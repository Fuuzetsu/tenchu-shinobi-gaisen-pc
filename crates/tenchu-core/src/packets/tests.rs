use super::*;
use tenchu_assets::model::{Corner, Face};

fn quad() -> Geometry {
    Geometry {
        vertices: vec![
            [-100, -100, 0],
            [100, -100, 0],
            [-100, 100, 0],
            [100, 100, 0],
        ],
        normals: vec![],
        faces: vec![Face {
            mode: 0x2d,
            flags: 1,
            corners: (0..4)
                .map(|vertex| Corner {
                    vertex,
                    normal: None,
                    color: [128; 3],
                    uv: [0; 2],
                })
                .collect(),
            texture: None,
            double_sided: false,
            translucent: false,
        }],
    }
}

#[test]
fn custom_and_sdk_draw_paths_disagree_on_the_fourth_vertex_and_lit_tail() {
    let mut mesh = quad();
    let matrix = Matrix {
        translation: [0, 0, 5000],
        ..Matrix::default()
    };
    mesh.vertices[3][2] = -6000;
    assert_eq!(draw_tmd(&mesh, &matrix, false), 40);
    assert_eq!(draw_tmd(&mesh, &matrix, true), 40);
    assert_eq!(sdk_plain(&mesh, &matrix).unwrap(), 0);
    mesh.vertices[3][2] = 0;
    let mut tail = mesh.faces[0].clone();
    tail.flags = 0;
    mesh.faces.push(tail);
    mesh.faces.push(mesh.faces[0].clone());
    assert_eq!(draw_tmd(&mesh, &matrix, false), 40);
    assert!(sdk_plain(&mesh, &matrix).is_err());
}

#[test]
fn adaptive_subdivision_counts_degenerate_edge_stitches() {
    let mut mesh = quad();
    let matrix = Matrix {
        translation: [0, 0, 1000],
        ..Matrix::default()
    };
    let mut params = Subdivision {
        depth: 1000,
        extent: [32; 2],
    };
    assert_eq!(params.count(&mesh, &matrix, 1).unwrap(), 4 * 40 + 4 * 32);
    params.extent = [16; 2];
    assert_eq!(
        params.count(&mesh, &matrix, 2).unwrap(),
        4 * (4 * 40 + 4 * 32) + 4 * 32
    );
    mesh.faces[0].mode = 0x25;
    mesh.faces[0].corners.pop();
    assert_eq!(params.count(&mesh, &matrix, 1).unwrap(), 7 * 32);
    mesh.faces[0].corners.swap(1, 2);
    assert_eq!(params.count(&mesh, &matrix, 1).unwrap(), 0);
}
