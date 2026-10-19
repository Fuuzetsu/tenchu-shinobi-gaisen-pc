use super::*;
use std::{fs, io::Read};
use tenchu_assets::executable::Executable;

#[test]
#[ignore = "requires TENCHU_RETAIL_EXE and TENCHU_CAMERA_TRACE; compares against the captured SDK matrix"]
fn view_matrix_matches_retail_capture_and_resets_on_zero_length() {
    let bytes =
        fs::read(std::env::var_os("TENCHU_RETAIL_EXE").expect("Set TENCHU_RETAIL_EXE")).unwrap();
    let exe = Executable::new(&bytes).unwrap();
    let math = Math {
        tables: exe.math_tables().unwrap(),
    };
    let mut camera = Camera::new(CameraTables::load(&exe).unwrap(), [0; 3], &math).unwrap();
    let mut capture = vec![0; 96 + 0x200000];
    fs::File::open(std::env::var_os("TENCHU_CAMERA_TRACE").expect("Set TENCHU_CAMERA_TRACE"))
        .unwrap()
        .read_exact(&mut capture)
        .unwrap();
    assert!(matches!(&capture[..8], b"ONKTR001" | b"ONKTR002"));
    assert_eq!(&capture[16..24], b"ONKRAM01");
    let ram = &capture[96..];
    let word = |at| i32::from_le_bytes(ram[at..at + 4].try_into().unwrap());
    let short = |at| i16::from_le_bytes(ram[at..at + 2].try_into().unwrap());
    assert_eq!(
        word(0xb8080 + 24),
        0,
        "rolled cinematic views are a separate path"
    );
    camera.position = std::array::from_fn(|i| word(0xb8080 + i * 4));
    camera.reference = std::array::from_fn(|i| word(0xb808c + i * 4));
    let expected = Matrix {
        rotation: std::array::from_fn(|i| {
            std::array::from_fn(|j| short(0xc6520 + (i * 3 + j) * 2))
        }),
        translation: std::array::from_fn(|i| word(0xc6534 + i * 4)),
    };
    camera.update_view(&math).unwrap();
    assert_eq!(camera.view, expected);
    for point in [[0; 3], [123, -456, 789], [800000, -900000, 1000000]] {
        camera.position = point;
        camera.reference = point;
        camera.update_view(&math).unwrap();
        assert_eq!(camera.view, Matrix::default());
    }
}
