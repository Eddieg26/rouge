use ecs::system::schedule::{Phase, Schedule};

pub struct Startup;
impl Phase for Startup {
    fn schedule() -> Schedule {
        let mut schedule = Schedule::from::<Startup>();
        schedule.add_child(Schedule::from::<PreInit>());
        schedule.add_child(Schedule::from::<Init>());
        schedule.add_child(Schedule::from::<PostInit>());
        schedule
    }
}

pub struct Init;
impl Phase for Init {}

pub struct PreInit;
impl Phase for PreInit {}

pub struct PostInit;
impl Phase for PostInit {}

pub struct First;
impl Phase for First {}

pub struct Fixed;
impl Phase for Fixed {
    fn schedule() -> Schedule {
        let mut schedule = Schedule::from::<Fixed>();
        schedule.add_child(Schedule::from::<PreFixedUpate>());
        schedule.add_child(Schedule::from::<FixedUpdate>());
        schedule.add_child(Schedule::from::<PostFixedUpdate>());
        schedule
    }
}
pub struct PreFixedUpate;
impl Phase for PreFixedUpate {}
pub struct FixedUpdate;
impl Phase for FixedUpdate {}
pub struct PostFixedUpdate;
impl Phase for PostFixedUpdate {}

pub struct PreUpdate;
impl Phase for PreUpdate {}
pub struct Update;
impl Phase for Update {}
pub struct PostUpdate;
impl Phase for PostUpdate {}

pub struct Last;
impl Phase for Last {}

pub struct PreExecute;
impl Phase for PreExecute {}

pub struct Execute;
impl Phase for Execute {
    fn schedule() -> Schedule {
        let mut schedule = Schedule::from::<Execute>();
        schedule.add_child(Schedule::from::<First>());
        schedule.add_child(Schedule::from::<Fixed>());
        schedule.add_child(Schedule::from::<PreUpdate>());
        schedule.add_child(Schedule::from::<Update>());
        schedule.add_child(Schedule::from::<PostUpdate>());
        schedule.add_child(Schedule::from::<Last>());
        schedule
    }
}

pub struct PostExecute;
impl Phase for PostExecute {}

pub struct Shutdown;
impl Phase for Shutdown {
    fn schedule() -> Schedule {
        let mut schedule = Schedule::from::<Shutdown>();
        schedule.add_child(Schedule::from::<PreExit>());
        schedule.add_child(Schedule::from::<Exit>());
        schedule.add_child(Schedule::from::<PostExit>());
        schedule
    }
}

pub struct Exit;
impl Phase for Exit {}

pub struct PreExit;
impl Phase for PreExit {}

pub struct PostExit;
impl Phase for PostExit {}

pub struct Extract;
impl Phase for Extract {}
