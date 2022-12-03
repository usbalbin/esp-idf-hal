use core::{marker::PhantomData, ptr};

use esp_idf_sys::{
    esp, mcpwm_gen_handle_t, mcpwm_gen_timer_event_action_t, mcpwm_generator_action_t,
    mcpwm_generator_action_t_MCPWM_GEN_ACTION_HIGH, mcpwm_generator_action_t_MCPWM_GEN_ACTION_KEEP,
    mcpwm_generator_action_t_MCPWM_GEN_ACTION_LOW,
    mcpwm_generator_action_t_MCPWM_GEN_ACTION_TOGGLE, mcpwm_generator_config_t,
    mcpwm_generator_config_t__bindgen_ty_1, mcpwm_generator_set_actions_on_timer_event,
    mcpwm_new_generator, mcpwm_oper_handle_t, mcpwm_timer_direction_t_MCPWM_TIMER_DIRECTION_DOWN,
    mcpwm_timer_direction_t_MCPWM_TIMER_DIRECTION_UP, mcpwm_timer_event_t_MCPWM_TIMER_EVENT_EMPTY,
    mcpwm_timer_event_t_MCPWM_TIMER_EVENT_FULL,
};

use crate::gpio::OutputPin;

use super::comparator::{Comparator, OptionalCmp};

pub struct NoGen;

impl OptionalGen for NoGen {}
pub trait OptionalGen {}

impl<G, CMPX, CMPY, P> OptionalGen for Generator<G, CMPX, CMPY, P>
where
    G: GeneratorChannel,
    CMPX: OnMatchCfg,
    CMPY: OnMatchCfg,
    P: OutputPin,
{
}

pub trait GeneratorChannel {
    const IS_A: bool;
}

pub struct GenA;
impl GeneratorChannel for GenA {
    const IS_A: bool = true;
}

pub struct GenB;
impl GeneratorChannel for GenB {
    const IS_A: bool = false;
}

// TODO: Allow OptionalOutputPin?
pub struct Generator<G, CMPX, CMPY, P: OutputPin> {
    channel: PhantomData<G>,
    cmp_x: PhantomData<CMPX>,
    cmp_y: PhantomData<CMPY>,
    pub(crate) _handle: mcpwm_gen_handle_t,
    pub(crate) _pin: P,
}

pub struct GeneratorConfig<G: GeneratorChannel, CMPX, CMPY, P> {
    _channel: PhantomData<G>,
    pub(crate) flags: mcpwm_generator_config_t__bindgen_ty_1,
    pub(crate) on_matches_cmp_x: CMPX,
    pub(crate) on_matches_cmp_y: CMPY,
    pub(crate) on_is_empty: CountingDirection,
    pub(crate) on_is_full: CountingDirection,
    pub(crate) pin: P,
}

pub struct NoGenCfg;

pub trait OptionalGenCfg {
    type Gen: OptionalGen;

    /// This is only to be used internally by esp-idf-hal
    unsafe fn init(
        self,
        operator_handle: mcpwm_oper_handle_t,
        cmp_x: Option<&mut Comparator>,
        cmp_y: Option<&mut Comparator>,
    ) -> Self::Gen;
}

impl OptionalGenCfg for NoGenCfg {
    type Gen = NoGen;

    unsafe fn init(
        self,
        _operator_handle: mcpwm_oper_handle_t,
        _cmp_x: Option<&mut Comparator>,
        _cmp_y: Option<&mut Comparator>,
    ) -> NoGen {
        NoGen
    }
}

impl<G: GeneratorChannel, CMPX: OnMatchCfg, CMPY: OnMatchCfg, P: OutputPin> OptionalGenCfg
    for GeneratorConfig<G, CMPX, CMPY, P>
{
    type Gen = Generator<G, CMPX, CMPY, P>;

    unsafe fn init(
        self,
        operator_handle: mcpwm_oper_handle_t,
        cmp_x: Option<&mut Comparator>,
        cmp_y: Option<&mut Comparator>,
    ) -> Self::Gen {
        let cfg = mcpwm_generator_config_t {
            gen_gpio_num: self.pin.pin(),
            flags: self.flags,
        };
        let mut gen = ptr::null_mut();
        unsafe {
            esp!(mcpwm_new_generator(operator_handle, &cfg, &mut gen)).unwrap();

            // TODO: "must be terminated by MCPWM_GEN_TIMER_EVENT_ACTION_END()"
            esp!(mcpwm_generator_set_actions_on_timer_event(
                gen,
                mcpwm_gen_timer_event_action_t {
                    direction: mcpwm_timer_direction_t_MCPWM_TIMER_DIRECTION_UP,
                    event: mcpwm_timer_event_t_MCPWM_TIMER_EVENT_EMPTY,
                    action: self.on_is_empty.counting_up.into(),
                }
            ))
            .unwrap();
            esp!(mcpwm_generator_set_actions_on_timer_event(
                gen,
                mcpwm_gen_timer_event_action_t {
                    direction: mcpwm_timer_direction_t_MCPWM_TIMER_DIRECTION_DOWN,
                    event: mcpwm_timer_event_t_MCPWM_TIMER_EVENT_EMPTY,
                    action: self.on_is_empty.counting_down.into(),
                }
            ))
            .unwrap();
            esp!(mcpwm_generator_set_actions_on_timer_event(
                gen,
                mcpwm_gen_timer_event_action_t {
                    direction: mcpwm_timer_direction_t_MCPWM_TIMER_DIRECTION_UP,
                    event: mcpwm_timer_event_t_MCPWM_TIMER_EVENT_FULL,
                    action: self.on_is_full.counting_up.into(),
                }
            ))
            .unwrap();
            esp!(mcpwm_generator_set_actions_on_timer_event(
                gen,
                mcpwm_gen_timer_event_action_t {
                    direction: mcpwm_timer_direction_t_MCPWM_TIMER_DIRECTION_DOWN,
                    event: mcpwm_timer_event_t_MCPWM_TIMER_EVENT_FULL,
                    action: self.on_is_full.counting_down.into(),
                }
            ))
            .unwrap();

            if let Some(cmp_x) = cmp_x {
                cmp_x.configure(&mut *gen, self.on_matches_cmp_x.to_counting_direction());
            }

            if let Some(cmp_y) = cmp_y {
                cmp_y.configure(&mut *gen, self.on_matches_cmp_y.to_counting_direction());
            }
        }

        Generator {
            channel: PhantomData,
            cmp_x: PhantomData,
            cmp_y: PhantomData,
            _handle: gen,
            _pin: self.pin,
        }
    }
}

pub trait GenInit {
    type Gen: OptionalGen;

    /// This is only to be used internally by esp-idf-hal
    unsafe fn init(self, operator_handle: mcpwm_oper_handle_t) -> Self::Gen;
}

impl<CMPX, CMPY> GenInit for (&mut CMPX, &mut CMPY, NoGenCfg)
where
    CMPX: OptionalCmp,
    CMPY: OptionalCmp,
{
    type Gen = NoGen;

    unsafe fn init(self, _operator_handle: mcpwm_oper_handle_t) -> Self::Gen {
        NoGen
    }
}

impl<G: GeneratorChannel, P> GeneratorConfig<G, CountingDirection, CountingDirection, P> {
    pub fn active_high(pin: P) -> Self {
        let mut result: Self = GeneratorConfig::empty(pin);

        result.on_is_empty.counting_up = GeneratorAction::SetHigh;
        if G::IS_A {
            result.on_matches_cmp_x.counting_up = GeneratorAction::SetLow;
        } else {
            result.on_matches_cmp_y.counting_up = GeneratorAction::SetLow;
        }
        result
    }

    pub fn active_low(pin: P) -> Self {
        let mut result: Self = GeneratorConfig::empty(pin);
        result.on_is_empty.counting_up = GeneratorAction::SetLow;
        if G::IS_A {
            result.on_matches_cmp_x.counting_up = GeneratorAction::SetHigh;
        } else {
            result.on_matches_cmp_y.counting_up = GeneratorAction::SetHigh;
        }
        result
    }
}

// TODO: Do we have any use for this?
impl<G, CMPX, CMPY, P> GeneratorConfig<G, CMPX, CMPY, P>
where
    G: GeneratorChannel,
    CMPX: OnMatchCfg,
    CMPY: OnMatchCfg,
{
    fn empty(pin: P) -> Self {
        let mut flags: mcpwm_generator_config_t__bindgen_ty_1 = Default::default();
        flags.set_invert_pwm(0);
        flags.set_io_loop_back(0);

        GeneratorConfig {
            _channel: PhantomData,
            flags,
            on_matches_cmp_x: OnMatchCfg::empty(),
            on_matches_cmp_y: OnMatchCfg::empty(),
            on_is_empty: CountingDirection {
                counting_up: GeneratorAction::Nothing,
                counting_down: GeneratorAction::Nothing,
            },
            on_is_full: CountingDirection {
                counting_up: GeneratorAction::Nothing,
                counting_down: GeneratorAction::Nothing,
            },
            pin,
        }
    }
}

pub struct NoCmpMatchConfig;

impl OnMatchCfg for NoCmpMatchConfig {
    fn empty() -> Self {
        NoCmpMatchConfig
    }

    fn to_counting_direction(self) -> CountingDirection {
        CountingDirection::empty()
    }
}

// TODO: Come up with better name
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CountingDirection {
    pub(crate) counting_up: GeneratorAction,
    pub(crate) counting_down: GeneratorAction,
}

impl CountingDirection {
    pub fn empty() -> Self {
        CountingDirection {
            counting_up: GeneratorAction::Nothing,
            counting_down: GeneratorAction::Nothing,
        }
    }
}

impl OnMatchCfg for CountingDirection {
    fn empty() -> Self {
        CountingDirection::empty()
    }

    fn to_counting_direction(self) -> CountingDirection {
        self
    }
}

pub trait OnMatchCfg {
    fn empty() -> Self;
    fn to_counting_direction(self) -> CountingDirection;
}

impl From<GeneratorAction> for mcpwm_generator_action_t {
    fn from(val: GeneratorAction) -> Self {
        match val {
            GeneratorAction::Nothing => mcpwm_generator_action_t_MCPWM_GEN_ACTION_KEEP,
            GeneratorAction::SetLow => mcpwm_generator_action_t_MCPWM_GEN_ACTION_LOW,
            GeneratorAction::SetHigh => mcpwm_generator_action_t_MCPWM_GEN_ACTION_HIGH,
            GeneratorAction::Toggle => mcpwm_generator_action_t_MCPWM_GEN_ACTION_TOGGLE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratorAction {
    Nothing,
    SetLow,
    SetHigh,
    Toggle,
}