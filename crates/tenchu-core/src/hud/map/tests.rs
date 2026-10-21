use super::*;
use crate::math::Position;
use crate::{items::Processor, oracle};
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; original map animation and item markers, including full-width coordinates and wrapped screen lines"]
fn original_map_callbacks_match() {
    let info = oracle::source("INFOVIEW.C");
    let item = oracle::source("ITEM.C");
    let effect = oracle::source("EFFECT.C");
    let functions = [
        oracle::function(
            &effect,
            "void DrawTargetS(long x, long y, long z, long color)",
        ),
        oracle::function(
            &item,
            "void draw_map_items_(s32 x, s32 z, MapPlacementType *placement)",
        ),
        oracle::function(&info, "static void PutMap(void)\n{"),
    ]
    .join("\n");
    let harness = [r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
#define long int32_t
typedef int32_t s32;typedef uint8_t u8;typedef struct {int vx,vy,vz;} VECTOR;
typedef struct {int scale_divisor,rotation,screen_x,screen_y;} MapPlacementType;
typedef struct {struct {struct {int t[3];}coord;}locate;} Model;
typedef struct {Model*model;} Human;
typedef struct {int proc,type;Human*owner;Model*locate;} Item;
typedef struct {int x,y,scalex,scaley;uint8_t r,g,b;} Sprite;
typedef struct {int attribute;int16_t x0,y0,x1,y1;uint8_t r,g,b;} GsLINE;
typedef struct {struct {uint8_t r0,g0,b0;}ply;} POLY_XF4;
#define setXY4(...) ((void)0)
#define RGB24(r,g,b) ((r)|((g)<<8)|((b)<<16))
enum {FIXED_TRUNC_BIAS=4095,FIXED_SHIFT=12,MAX_ITEMS=30,ITEM_GOSHIKIMAI=8,
GPU_BLEND_SUBTRACT=2,SCREEN_W=320,SCREEN_H=240,PUTMAP_OPEN=0,PUTMAP_SLIDE_IN=1,PUTMAP_SHOWN=2,SE_MAP_OPEN=0x27,FIXED_ONE=4096,DEPTH_LIMIT=4096};
static int Sin,Cos,PutMapMode,MapSlideX,MapSlideY,StageID;
static Model model[31];static Human human,other;static struct {Human*Owner;} CamState;
static Item items[30];static MapPlacementType MapPlacement[1];static Sprite MapImage;
static struct {int org[4];} ot,*OTablePt=&ot;static POLY_XF4 poly;
static int sprites[2][8],lines[62][7],ns,nl,bytes,sound;
static int rsin(int a){return Sin;}static int rcos(int a){return Cos;}
static void* GsGetWorkBase(void){return &poly;}static void GsSetWorkBase(void*p){}
static void SetPolyXF4(POLY_XF4*p,int blend){assert(blend==2);bytes+=32;}
static void AddXF4(void*p,POLY_XF4*q){}
static void SoundEx(void*p,int id){assert(!p);sound=id;}
static void GsSortLine(GsLINE*l,void*p,int depth){assert(depth==0);int*q=lines[nl++];q[0]=l->x0;q[1]=l->y0;q[2]=l->x1;q[3]=l->y1;q[4]=l->r;q[5]=l->g;q[6]=l->b;bytes+=16;}
static void GsSortSprite(Sprite*s,void*p,int depth){int*q=sprites[ns++];q[0]=(int16_t)s->x;q[1]=(int16_t)s->y;q[2]=s->r;q[3]=s->g;q[4]=s->b;q[5]=depth;q[6]=s->scalex;q[7]=s->scaley;bytes+=24;}
"#, &functions, r#"
int main(void){int a[12];while(scanf("%d",a)==1){for(int i=1;i<12;i++)if(scanf("%d",a+i)!=1)return 2;
PutMapMode=a[0];MapSlideX=a[1];MapSlideY=0;Sin=a[8];Cos=a[9];StageID=0;MapPlacement[0]=(MapPlacementType){a[4],a[5],a[6],a[7]};
model[30].locate.coord.t[0]=a[2];model[30].locate.coord.t[2]=a[3];human.model=model+30;CamState.Owner=&human;
for(int i=0;i<30;i++){model[i].locate.coord.t[0]=a[2]+i*317;model[i].locate.coord.t[2]=a[3]-i*991;items[i]=(Item){(i+a[10])%2,i%3?8:4,(i+a[11])%4?&human:&other,model+i};}
MapImage=(Sprite){.r=128,.g=128,.b=128};ns=nl=bytes=0;sound=-1;PutMap();
printf("%d %d %d %d %d %d %d",PutMapMode,MapSlideX,poly.ply.r0,ns,nl,bytes,sound);
for(int i=0;i<ns;i++)for(int j=0;j<8;j++)printf(" %d",sprites[i][j]);
for(int i=0;i<nl;i++)for(int j=0;j<7;j++)printf(" %d",lines[i][j]);puts("");
}return 0;}
"#].concat();
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    let root = game.player().root_model;
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..1024i32 {
        let mode = case as u8 % 3;
        let slide = [160, 120, 80, 40, 0, -40, -200, 32768][case as usize / 3 % 8];
        let [x, z] = match case % 4 {
            0 => [i32::MIN, i32::MAX],
            1 => [-45_318, -43_068],
            2 => [case.wrapping_mul(7_918_731), case.wrapping_mul(-3_171_135)],
            _ => [-1, 1],
        };
        let placement = MapPlacement {
            divisor: [0, -1, 1, 790, 487, 1344, i32::MAX][case as usize / 7 % 7],
            rotation: case * 311,
            screen: [case * 101 - 50_000, case * -73 + 40_000],
        };
        let [sin, cos] = game.math.pair(placement.rotation as i16);
        writeln!(
            input,
            "{mode} {slide} {x} {z} {} {} {} {} {sin} {cos} {} {}",
            placement.divisor,
            placement.rotation,
            placement.screen[0],
            placement.screen[1],
            case % 2,
            case % 4
        )
        .unwrap();
        game.mission.map_placement = placement;
        game.scene.models[root].local.translation = Position::new(x, 0, z);
        for (i, item) in game.items.slots.iter_mut().enumerate() {
            item.processor = ((i as i32 + case % 2) % 2 != 0).then_some(Processor::Pickup);
            item.kind = if i % 3 != 0 { 8 } else { 4 };
            item.owner = Some(if (i as i32 + case % 4) % 4 != 0 {
                player
            } else {
                usize::MAX
            });
            game.scene.models[item.model].local.translation = Position::new(
                x.wrapping_add(i as i32 * 317),
                0,
                z.wrapping_sub(i as i32 * 991),
            );
        }
        let mut map = Map {
            mode,
            slide,
            ..Map::default()
        };
        let mut sprites = Vec::new();
        game.drawing.bytes = 0;
        game.effects.sounds.clear();
        map.step(
            true,
            game.roster.get(player),
            &game.items,
            &game.scene,
            &mut sprites,
            &mut Context {
                mission: &game.mission,
                math: &game.math,
                effects: &mut game.effects,
                drawing: &mut game.drawing,
            },
        );
        let mut row = vec![
            i32::from(map.mode),
            map.slide,
            i32::from(map.dim.unwrap()),
            sprites.len() as i32,
            map.lines.len() as i32,
            game.drawing.bytes as i32,
            game.effects.sounds.last().map_or(-1, |s| i32::from(s.1)),
        ];
        for s in sprites {
            row.extend(s.xy.map(i32::from));
            row.extend(s.color.map(i32::from));
            row.push(i32::from(s.depth));
            row.extend(s.scale.map(i32::from));
        }
        for line in map.lines {
            row.extend(line.points.into_iter().flatten().map(i32::from));
            row.extend(line.color.map(i32::from));
        }
        expected.push(row);
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (case, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(
            actual,
            expected,
            "case {case}: {}",
            input.lines().nth(case).unwrap()
        );
    }
}
