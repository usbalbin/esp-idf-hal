#[derive(Clone, Copy, Debug)]
struct TimerConfig {
    frequency: Hertz,
    resolution: Hertz,
    counter_mode: CounterMode,

    // TODO
    // on_full,
    // on_empty,
    // on_stop,
}

impl TimerConfig {
    #[must_use]
    pub fn resolution(mut self, resolution: impl Into<Hertz>) -> Self {
        self.resolution = resolution.into();
        self
    }

    /// Frequency which the operator will run at, can also be changed live later
    #[must_use]
    pub fn frequency(mut self, frequency: impl Into<Hertz>) -> Self {
        self.frequency = frequency.into();
        self
    }

    #[must_use]
    pub fn counter_mode(mut self, counter_mode: CounterMode) -> Self {
        self.counter_mode = counter_mode;
        self
    }
}

//TODO
//impl<G, H> TimerConfig<NoCb, G, H> {
    //#[must_use]
    //pub fn on_full(mut self, on_full: CB) -> Self<CB, _, _> {
    //    self.on_full = on_full;
    //    self
    //}
//}

struct Timer<U: Unit, T: HwTimer<U>> {
    handle: mcpwm_timer_handle_t,
    _timer: T,
}

impl Timer {
    pub fn new(timer: T, config: TimerConfig) -> Self {
        let config = mcpwm_timer_config_t {
            resolution
        };
        let mut handle = ptr::null();
        mcpwm_new_timer(config, &mut handle);

        // TODO: note that this has to be called before mcpwm_timer_enable
        // mcpwm_timer_register_event_callbacks()

        mcpwm_timer_enable();

        Self { handle, _timer: timer }
    }
    /// Set PWM frequency
    pub fn set_frequency(&mut self, frequency: Hertz) -> Result<(), EspError> {
        todo!()
    }

    /// Get PWM frequency
    pub fn get_frequency(&self) -> Hertz {
        todo!()
    }

    pub fn timer(&self) -> mcpwm_timer_t {
        T::timer()
    }

    pub fn release(self) -> T {
        let Self {
            _timer,
            handle
        } = self;
        mcpwm_del_timer(handle);
        _timer
    }

    fn into_connection(timer: T) -> TimerConnection<U, T, NoOperator, NoOperator, NoOperator> {
        TimerConnection::new(timer)
    }
}

impl Drop for Timer {
    fn drop(self) {
        mcpwm_del_timer(self.handle)
    }
}

/// Counter mode for operator's timer for generating PWM signal
// TODO: For UpDown, frequency is half of MCPWM frequency set
#[derive(Clone, Copy, Debug)]
pub enum CounterMode {
    /// Timer is frozen or paused
    #[cfg(not(esp_idf_version = "4.3"))]
    Frozen,
    /// Edge aligned. The counter will start from its lowest value and increment every clock cycle until the period is reached.
    ///
    /// The wave form will end up looking something like the following:
    /// ```
    ///       start, counter = 0                     reset, counter = period
    ///         |                                       |
    ///         |                                       |*--- start, counter = 0
    ///         v <----  duty  ----> .                  v|
    ///         .                    .                  .v
    ///         .--------------------.                  ..----
    ///         |       Active       |                  .|
    ///         |                    |                  .|
    ///         |                    |     Not active   .|
    ///         -                    ---------------------
    /// ```
    Up,

    /// Edge aligned. The counter will start from its highest value, period and decrement every clock cycle until the zero is reached
    ///
    /// The wave form will end up looking something like the following:
    /// ```
    ///       start, counter = period                   reset, counter = 0
    ///         |                                         |
    ///         |                                         |*--- start, counter = period
    ///         v                    .                    v|
    ///         .                    . <----  duty  ----> .v
    ///         .                    .--------------------..
    ///         .       Active       |                    |.
    ///         .                    |                    |.
    ///         .     Not active     |      Active        |.
    ///         ----------------------                    ----
    /// ```
    Down,

    /// Symmetric mode. The counter will start from its lowest value and increment every clock cycle until the period is reached
    ///
    /// The wave form will end up looking something like the following:
    /// ```
    ///                                             change count dir to decrement, counter = period
    ///       start, counter = 0, incrementing          |                                     change count dir to increment, counter = 0
    ///         |                                       |                                        |
    ///         |                                       |*--- counter = period                   |*----- start, counter = 0, incrementing
    ///         v <----  duty  ----> .                  v|                  . <----  duty  ----> ||
    ///         .                    .                  .v                  .                    vv
    ///         ---------------------.                  ..                  .-------------------------------------------.                  ..                  .--
    ///                 Active       |                  ..                  |        Active                Active       |                  ..                  |
    ///                              |                  ..                  |                                           |                  ..                  |
    ///                              |     Not active   ..    Not active    |                                           |     Not active   ..    Not active    |
    ///                              ----------------------------------------                                           ----------------------------------------
    /// ```
    /// NOTE: That in this mode, the frequency will be half of that specified
    UpDown,
}

impl From<CounterMode> for mcpwm_counter_type_t {
    fn from(val: CounterMode) -> Self {
        match val {
            #[cfg(not(esp_idf_version = "4.3"))]
            CounterMode::Frozen => mcpwm_counter_type_t_MCPWM_FREEZE_COUNTER,
            CounterMode::Up => mcpwm_counter_type_t_MCPWM_UP_COUNTER,
            CounterMode::Down => mcpwm_counter_type_t_MCPWM_DOWN_COUNTER,
            CounterMode::UpDown => mcpwm_counter_type_t_MCPWM_UP_DOWN_COUNTER,
        }
    }
}