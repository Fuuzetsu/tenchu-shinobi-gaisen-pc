#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
typedef int16_t s16;
typedef uint16_t u16;
typedef uint8_t u8;
typedef int32_t s32;
typedef int motion_id;
typedef int TItemType;
typedef struct { int32_t vx, vy, vz, pad; } VECTOR;
typedef struct { int16_t vx, vy, vz, pad; } SVECTOR;
typedef struct { int id, attribute; } ModelType;
typedef struct { ModelType *object[15]; } ModelArchiveType;
typedef struct { int mid, sweep, orderspd, sidespd, time; } MotionDataType;
typedef struct { short mid, count, loop; int mask; MotionDataType *motion; } MotionManager;
typedef struct Humanoid {
    short type, life, lifemax, status, height, wpatk, active_item;
    u16 attribute;
    u8 item[26];
    struct { short time; } pad;
    struct { int height, attrib; } map;
    ModelArchiveType *model;
    MotionManager *motion;
    VECTOR *locate;
    SVECTOR *rotate;
    SVECTOR vector;
} Humanoid;
typedef struct { ModelType *model; Humanoid *common; VECTOR position; } Conflict;
typedef struct { short power; } BattleType;
static Humanoid humans[3], *Me_MOTION_C, *StagePlayer, *DeadHumanoid;
static ModelType models[2][15];
static ModelArchiveType archives[2];
static MotionManager managers[2];
static VECTOR locations[2], abs_point;
static SVECTOR rotations[2], ConflictDistance;
static Conflict ConflictObject[80];
static int ConflictObjects;
static BattleType BattleDB[1];
static MotionManager *dtM;
static SVECTOR *dtV, *dtR;
static VECTOR *dtL;
static int motID, motMODE, MotionUpdateMode, ActionHalt, gNannido, EngageLevel;
static int ARMOUR_EQUIPPED_, Criticals, Murders, FriendHits, GlobalAreaMap;
static int calls, direction, alert, feedback, blood_count, bleed_count, impacts, played, sound_count, sounds[64];
static uint32_t seed;
#define rand rng
static int rng(void) { seed = seed * 0x41c64e6d + 12345; calls++; return (seed >> 16) & 32767; }
#define PadShockAR(...) ((void)0)
#define SetCameraMode(...) ((void)0)
#undef CONFLICT_OWNER_ITEM
#define CONFLICT_OWNER_ITEM 0
#define DAMAGE_LAUNCH_SPEED 70
#define ACTIVE_ITEM_PROTECTION ITEM_GOSIN
#define COLOR_YELLOW 0xffff00
#define COLOR_GRAY 0x808080
#define RGB24(r,g,b) (((r)<<16)|((g)<<8)|(b))
#define FIXED_ONE 4096
#define IMPACT_SPRITE_HIT 2
#define IMPACT_SPRITE_FLASH 1
#define DAMAGE_MOTION_LAUNCH_TIER 3
#define DAMAGE_MOTION_FROM_BEHIND_OFFSET 4
#define N_DAMAGE_MOTIONS 8
#undef MOTION_UPDATE_NOT_FOUND
#define MOTION_UPDATE_NOT_FOUND -1
#define CONFLICT_NONE -1
#define MOTION_ID_NONE -1
#define MODEL_PART_BEAST_HAND_0 2
#define ATTACK_CANCEL_CONFLICTS 1
#define ATTACK_CANCEL_AFTERIMAGES 2
#define ATTACK_CANCEL_ALL 3
#define SET_MOTION(mid,mode) (motID=(mid),motMODE=(mode))
#define SELECT_RETURN_MOTION() SET_MOTION((Me_MOTION_C->attribute & ATTR_WEAPON_DRAWN) ? MOT_ENGAGE_STANCE : MOT_NORMAL, MOTION_MOVE_APPLY)
#define DISPOSE_WEAPON_AFTERIMAGE(...) ((void)0)
static int GetDirection(int x,int z,int yaw) { return direction; }
static int GetAreaMapPassage(int map,VECTOR *p,SVECTOR *step,int count) { return 0; }
static int GetAttackDBID(Humanoid *h,int mid) { return 0; }
static int item_kind;
static int GetItemType(int i) { return models[1][13].id == i ? item_kind : 0; }
static int GetConflictResult(ModelType *m,int i) { return -1; }
static void ReqItemDefault(Humanoid *h,int item) { abort(); }
static void reset_alert_duration(void) { alert = gNannido == 2 ? 600 : 300; }
static void ReqLifeBar(Humanoid *h) { feedback = h - humans; }
static void Sound(Humanoid *h,int id) { sounds[sound_count++] = (h-humans)*256+id; }
static VECTOR *GetAbsolutePosition(ModelType *m,int x,int y,int z) { return &abs_point; }
static void SetBlood(VECTOR *p,short n,short time) { blood_count += n; while(n--) for(int i=0;i<7;i++) rng(); }
static void SetBleed(VECTOR *p,SVECTOR *v,int time,int color) { bleed_count++; }
static void SetImpact(VECTOR *p,short size,short type) { rng(); impacts++; }
static void set_impact_ex_(VECTOR *p,void *parent,short start,short end,int a,int b,int rot,int speed,int time,int type) { impacts++; }
static void MoveHumanoid(Humanoid *h,short forward,short side) {
    int a = h->rotate->vy;
    int s = Trig[abs(a) & 4095][0], c = Trig[abs(a) & 4095][1];
    if (a < 0) s = -s;
    s = -s; c = -c;
    h->vector.vx = (s * forward - c * side) >> 12;
    h->vector.vz = (c * forward + s * side) >> 12;
}
static int UpdateMotion(MotionManager *m,int id) {
    if (m->mid == id) return 0;
    for (int i=0;i<N_MOTIONS;i++) if (Motions[i].mid == id) {
        m->mid = id; m->motion = &Motions[i]; m->count = Motions[i].sweep; m->loop = 0; return 1;
    }
    return -1;
}
static void PlayMotion(MotionManager *m,int pose) {
    played++;
    if (m->loop < 0) return;
    m->count++;
    if (m->count > m->motion->time) { m->count=0; m->loop++; }
}
static int SetNowMotion(Humanoid *h,int mid,int move) {
    if (h->status == STAT_DEAD && h->motion->loop == -1) return 0;
    if (UpdateMotion(h->motion,mid) == -1) return 0;
    h->status = (int8_t)(mid >> 8);
    if (move != 0) MoveHumanoid(h,h->motion->motion->orderspd,h->motion->motion->sidespd);
    return 1;
}
static void SetNowMotionUnlessCva(void) { SetNowMotion(Me_MOTION_C,motID,motMODE); motMODE = -1; }
/* ORIGINAL_ROUTINES */
static void AttackCancelControl(s16 mode) { ClearAttackEffects(mode); }
int main(void) {
    unsigned initial_seed;
    int hp,kind,attr,mid,count,loops,height,item,power,difficulty,role,enemy_kind,enemy_item,armor,cleanup,yaw,dx,dz,dir,water;
    while(scanf("%u%d%d%d%d%d%d%d%d%d%d%d%d%d%d%d%d%d%d%d%d%d", &initial_seed,&hp,&kind,&attr,&mid,&count,&loops,&height,&item,&power,&difficulty,&role,&enemy_kind,&enemy_item,&armor,&cleanup,&yaw,&dx,&dz,&dir,&item_kind,&water)==22) {
        memset(humans,0,sizeof(humans)); memset(models,0,sizeof(models)); memset(managers,0,sizeof(managers)); memset(ConflictObject,0,sizeof(ConflictObject));
        memset(locations,0,sizeof(locations)); memset(rotations,0,sizeof(rotations));
        seed=initial_seed; calls=0; direction=dir; alert=0; feedback=-1; blood_count=bleed_count=impacts=played=sound_count=0;
        Criticals=Murders=FriendHits=0; ActionHalt=0; DeadHumanoid=0; MotionUpdateMode=0;
        for(int i=0;i<2;i++) {
            for(int j=0;j<15;j++) { models[i][j].id=-1; archives[i].object[j]=&models[i][j]; }
            humans[i].model=&archives[i]; humans[i].motion=&managers[i]; humans[i].locate=&locations[i]; humans[i].rotate=&rotations[i]; humans[i].wpatk=WEAPON_KIND;
            managers[i].mid=-1; UpdateMotion(&managers[i],MOT_ATTACK);
        }
        Me_MOTION_C=&humans[0]; StagePlayer=&humans[role];
        humans[0].life=hp; humans[0].lifemax=100; humans[0].height=2000; humans[0].type=kind; humans[0].attribute=attr;
        humans[0].active_item=item; humans[0].map.height=height; humans[0].map.attrib=water; humans[0].pad.time=19;
        UpdateMotion(&managers[0],mid); managers[0].mid=mid; managers[0].count=count; managers[0].loop=loops; managers[0].mask=0x1234;
        humans[0].status=(int8_t)(mid>>8); humans[0].vector=(SVECTOR){21,7,-31,2};
        humans[1].status=STAT_ATTACK; humans[1].type=enemy_kind; humans[1].active_item=enemy_item; humans[1].vector=(SVECTOR){99,8,88,0};
        rotations[0].vy=yaw; locations[1].vx=dx; locations[1].vz=dz;
        BattleDB[0].power=power; gNannido=difficulty; EngageLevel=3-difficulty; ARMOUR_EQUIPPED_=armor;
        ConflictObjects=cleanup ? 5 : 3;
        ConflictObject[0]=(Conflict){&models[0][0],&humans[0],{0}}; models[0][0].id=0;
        ConflictObject[1]=(Conflict){&models[1][0],&humans[1],{0}}; models[1][0].id=1;
        if(cleanup) {
            ConflictObject[2]=(Conflict){&models[0][13],&humans[0],{0}}; models[0][13].id=2;
            ConflictObject[3]=(Conflict){&models[0][14],&humans[0],{0}}; models[0][14].id=3;
        }
        int hit=cleanup?4:2;
        ConflictObject[hit]=(Conflict){&models[1][13],&humans[1],{0}}; models[1][13].id=hit;
        humans[0].vector.pad=hit;
        if(item_kind>=0) ConflictObject[hit].common=0;
        ConflictDistance=(SVECTOR){dx,0,dz,0};
        dtM=&managers[0]; dtV=&humans[0].vector; dtR=&rotations[0]; dtL=&locations[0]; motID=mid; motMODE=-1;
        DamageControl();
        printf("%d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d", humans[0].life,humans[0].status,managers[0].mid,managers[0].count,managers[0].loop,
            dtV->vx,dtV->vy,dtV->vz,dtR->vy,managers[1].loop,humans[1].vector.vx,humans[1].vector.vz,
            calls,alert,feedback,Criticals,Murders,FriendHits,ConflictObjects,DeadHumanoid?0:-1,
            humans[0].pad.time,managers[0].mask,blood_count,bleed_count,impacts);
        printf(" %d %d",humans[0].item[1],sound_count); for(int i=0;i<sound_count;i++) printf(" %d",sounds[i]); puts("");
    }
}
