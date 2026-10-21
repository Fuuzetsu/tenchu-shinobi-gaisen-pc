use super::*;

#[test]
fn square_probes_distinguish_walls_from_corners() {
    let map = flat_map();
    let mut cursor = Cursor::new(&map);
    for (position, mask) in [
        ([0, 0, -9900], 3),
        ([9900, 0, 0], 10),
        ([0, 0, 9900], 12),
        ([-9900, 0, 0], 5),
        ([-9900, 0, -9900], 7),
        ([9900, 0, 9900], 14),
    ] {
        let probe = vector(&map, &mut cursor, position.into(), 300, 0).unwrap();
        assert_eq!(probe.vector, mask, "{position:?}");
        assert_eq!(probe.level, 0);
        assert_eq!((probe.angle_low, probe.angle_high), (0, 0));
    }
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_RETAIL_EXE and TENCHU_DECOMP; original movement probe with executable directions and recorded terrain-query responses"]
fn movement_probes_match_original_c_and_retail_directions() {
    use crate::{oracle, random::Random};
    use std::fmt::Write;
    use tenchu_assets::executable::Executable;

    let executable =
        std::fs::read(std::env::var_os("TENCHU_RETAIL_EXE").expect("Set TENCHU_RETAIL_EXE"))
            .unwrap();
    let executable = Executable::new(&executable).unwrap();
    let directions: [[i32; 2]; 4] = std::array::from_fn(|i| {
        let bytes = executable.bytes(0x800867d4 + i as u32 * 4, 4).unwrap();
        [
            i16::from_le_bytes(bytes[..2].try_into().unwrap()).into(),
            i16::from_le_bytes(bytes[2..].try_into().unwrap()).into(),
        ]
    });
    let source = oracle::source("CONFLICT.C");
    let function = oracle::function(&source, "long GetAreaMapVector(").replace("long", "int32_t");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
typedef uint32_t u32; typedef uint16_t u16;
typedef int AreaMapType;
typedef struct {{int32_t vx,vy,vz,pad;}} VECTOR;
typedef struct {{int32_t level,height,attrib,area,index,angleH,angleL,vector;}} MapVector;
enum {{AREA_LEVEL_REUSE_CACHED=16,AREA_LEVEL_ALLOW_DEEP=4,MAP_BUOYANT=2,
       MAP_PROBE_ALL=15,MAP_SLOPE_X=0x4000,MAP_SLOPE_Z=0x8000,N_MAP_PROBE_DIRECTIONS=4}};
#define LEVEL_NONE INT32_MIN
static int16_t direction[4][2];
static int FieldAttrib,FieldArea,FieldIndex,AreaMapLastY;
static int responses[5][5],queries[5][4],n;
static int32_t GetAreaMapLevel(AreaMapType*a,int x,int y,int z,int16_t mode) {{
 assert(!a && n<5); int*q=queries[n]; q[0]=x;q[1]=y;q[2]=z;q[3]=mode;
 int*r=responses[n++];FieldAttrib=r[1];FieldArea=r[2];FieldIndex=r[3];AreaMapLastY=r[4];return r[0];
}}
{function}
int main(void) {{
 for(int i=0;i<4;i++)for(int j=0;j<2;j++)if(scanf("%hd",&direction[i][j])!=1)return 2;
 int a[5];while(scanf("%d",a)==1){{
  for(int i=1;i<5;i++)if(scanf("%d",a+i)!=1)return 2;
  for(int i=0;i<5;i++)for(int j=0;j<5;j++)if(scanf("%d",&responses[i][j])!=1)return 2;
  n=0;MapVector m={{0}};VECTOR p={{a[0],a[1],a[2],0}};
  int result=GetAreaMapVector(0,&m,&p,a[3],a[4]);
  printf("%d %d %d %d %d %d %d %d %d %d %d %d %d %d",result,m.level,m.height,m.attrib,m.vector,m.angleL,m.angleH,m.area,m.index,FieldAttrib,FieldArea,FieldIndex,AreaMapLastY,n);
  for(int i=0;i<n;i++)for(int j=0;j<4;j++)printf(" %d",queries[i][j]);puts("");
 }}return 0;
}}
"#
    );
    let game = oracle::game(0, 0);
    let map = &game.mission.terrain;
    let nodes: Vec<_> = map
        .rows
        .iter()
        .flat_map(|row| (0..row.count).map(move |i| map.node(row.list + i * 16).unwrap()))
        .collect();
    let mut random = Random::new(46);
    let mut input = String::new();
    for direction in directions {
        writeln!(input, "{} {}", direction[0], direction[1]).unwrap();
    }
    let mut expected = Vec::new();
    for case in 0..4096 {
        let node = nodes[random.next15() as usize % nodes.len()];
        let [x1, z1, x2, z2] = node.bounds.map(|n| i32::from(n) * 10);
        let width = [0, 100, 300, 500, 1000, 2000][case % 6];
        let position = [
            [x1, x2, (x1 + x2) / 2][case % 3] + random.next15() % 601 - 300,
            i32::from(node.y) * 10 + [-1500, -1000, -20, 0, 20, 1000][case / 3 % 6],
            [z1, z2, (z1 + z2) / 2][case / 18 % 3] + random.next15() % 601 - 300,
        ];
        let mode = [0, 1, 4, 5, 8, 16, 17, 20, 21][case / 54 % 9];
        let initial = Cursor::new(map);
        let mut cursor = initial;
        let mut queries = Vec::new();
        let mut responses = Vec::new();
        for i in 0..5 {
            let (p, m) = if i == 0 {
                (position, mode & !REUSE_CACHED)
            } else {
                let [dx, dz] = directions[i - 1];
                (
                    [
                        position[0] + dx * width,
                        position[1],
                        position[2] + dz * width,
                    ],
                    mode,
                )
            };
            let floor = level(map, &mut cursor, p.into(), m).unwrap();
            queries.extend([p[0], p[1], p[2], i32::from(m)]);
            responses.push([
                floor,
                i32::from(cursor.attribute),
                cursor.area as i32,
                cursor.row as i32,
                cursor.last_y,
            ]);
            if i == 0 && floor == NONE && mode & ALLOW_DEEP == 0 {
                break;
            }
        }
        writeln!(
            input,
            "{} {} {} {width} {mode}",
            position[0], position[1], position[2]
        )
        .unwrap();
        for i in 0..5 {
            for value in responses.get(i).unwrap_or(&[0; 5]) {
                write!(input, "{value} ").unwrap();
            }
        }
        input.push('\n');
        let mut actual_cursor = initial;
        let probe = vector(map, &mut actual_cursor, position.into(), width, mode).unwrap();
        let mut row = vec![
            probe.level,
            probe.level,
            probe.height,
            i32::from(probe.attribute),
            i32::from(probe.vector),
            i32::from(probe.angle_low),
            i32::from(probe.angle_high),
            probe.area as i32,
            probe.row as i32,
            i32::from(actual_cursor.attribute),
            actual_cursor.area as i32,
            actual_cursor.row as i32,
            actual_cursor.last_y,
            responses.len() as i32,
        ];
        row.extend(queries);
        expected.push(row);
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (case, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(actual, &expected, "case {case}");
    }
}
