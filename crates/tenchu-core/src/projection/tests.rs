use super::*;
use crate::math::Position;

#[test]
fn saturated_depth_and_screen_flags_have_distinct_rejection_rules() {
    let matrix = Matrix {
        translation: Position::new(0, 0, 40000),
        ..Matrix::default()
    };
    let p = point(&matrix, [0; 3]);
    assert_ne!(p.flags & DRAW_TMD_REJECT, 0);
    assert_eq!(p.flags >> 31, 0);
    let p = point(
        &Matrix {
            translation: Position::new(0, 0, 200),
            ..Matrix::default()
        },
        [i16::MAX, i16::MIN, 0],
    );
    assert_eq!(p.xy, [1023, -1024]);
    assert_eq!(p.flags & 0x6000, 0x6000);
}

#[test]
#[ignore = "requires TENCHU_GTE_ORACLE; standalone BizHawk/Mednafen arithmetic oracle"]
fn integer_projection_matches_independent_gte_kernel() {
    use std::{
        io::{Read, Write},
        process::{Command, Stdio},
    };
    let mut input = Vec::new();
    let mut expected = Vec::new();
    let mut seed = 1u32;
    let mut next = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };
    for case in 0..67584 {
        let mut matrix = Matrix::default();
        let vertices = if case < 2048 {
            matrix.rotation = std::array::from_fn(|_| std::array::from_fn(|_| next() as i16));
            matrix.translation = Position::from_fn(|_| next() as i32);
            std::array::from_fn(|_| std::array::from_fn(|_| next() as i16))
        } else {
            matrix.translation.z = case - 2048;
            [[-100, -100, 0], [100, -100, 0], [-100, 100, 0]]
        };
        let tri = case & 1 == 0;
        let mut words = [0u32; 39];
        words[0] = if tri { 0x280030 } else { 0x180001 };
        for (i, value) in matrix.rotation.into_iter().flatten().enumerate() {
            words[1 + i / 2] |= u32::from(value as u16) << (16 * (i % 2));
        }
        for i in 0..3 {
            words[6 + i] = matrix.translation[i] as u32;
            words[33 + 2 * i] =
                u32::from(vertices[i][0] as u16) | (u32::from(vertices[i][1] as u16) << 16);
            words[34 + 2 * i] = u32::from(vertices[i][2] as u16);
        }
        words[27] = 300;
        words[28] = (-0x7ef4i32) as u32;
        words[29] = 0x2f282e0;
        for word in words {
            input.extend_from_slice(&word.to_le_bytes());
        }
        let mut output = [0u32; 11];
        let last = if tri {
            let (points, flags) = triangle(&matrix, vertices);
            for i in 0..3 {
                output[i] =
                    u32::from(points[i].xy[0] as u16) | (u32::from(points[i].xy[1] as u16) << 16);
                output[i + 3] = u32::from(points[i].z);
            }
            output[6] = flags;
            output[7] = area(points) as u32;
            points[2]
        } else {
            let p = point(&matrix, vertices[0]);
            output[2] = u32::from(p.xy[0] as u16) | (u32::from(p.xy[1] as u16) << 16);
            output[5] = u32::from(p.z);
            output[6] = p.flags;
            p
        };
        output[8..].copy_from_slice(&last.mac.map(|v| v as u32).to_array());
        expected.push(output);
    }
    let mut process =
        Command::new(std::env::var_os("TENCHU_GTE_ORACLE").expect("Set TENCHU_GTE_ORACLE"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
    let mut stdin = process.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(&input).unwrap());
    let mut output = Vec::new();
    process
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut output)
        .unwrap();
    writer.join().unwrap();
    assert!(process.wait().unwrap().success());
    assert_eq!(output.len(), expected.len() * 44);
    for (case, (actual, expected)) in output.chunks_exact(44).zip(expected).enumerate() {
        let actual: Vec<_> = actual
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        assert_eq!(actual, expected, "projection case {case}");
    }
}
