use esp_idf_sys::{
    esp, mcpwm_comparator_set_compare_value, mcpwm_oper_handle_t, mcpwm_operator_config_t,
    mcpwm_operator_config_t__bindgen_ty_1, mcpwm_operator_connect_timer, mcpwm_timer_handle_t,
    EspError, ESP_ERR_INVALID_ARG,
};

use crate::mcpwm::Group;

use super::{comparator::Comparator, generator::Generator, OperatorConfig};

use core::{marker::PhantomData, ptr};

pub struct OPERATOR<const N: u8, G: Group> {
    _ptr: PhantomData<*const ()>,
    _group: PhantomData<G>,
}

impl<const N: u8, G: Group> OPERATOR<N, G> {
    /// # Safety
    ///
    /// Care should be taken not to instnatiate this peripheralinstance, if it is already instantiated and used elsewhere
    #[inline(always)]
    pub unsafe fn new() -> Self {
        Self {
            _ptr: PhantomData,
            _group: PhantomData,
        }
    }
}

unsafe impl<const N: u8, G: Group> Send for OPERATOR<N, G> {}

impl<const N: u8, G: Group> crate::peripheral::sealed::Sealed for OPERATOR<N, G> {}

impl<const N: u8, G: Group> crate::peripheral::Peripheral for OPERATOR<N, G> {
    type P = Self;

    #[inline]
    unsafe fn clone_unchecked(&mut self) -> Self::P {
        Self { ..*self }
    }
}

// TODO: How do we want syncing to fit in to this?
// TODO: How do we want carrier to fit into this?
// TODO: How do we want capture to fit into this?

/// Motor Control operator abstraction
///
/// Every Motor Control module has three operators. Every operator can generate two output signals called A and B.
/// A and B share the same timer and thus frequency and phase but can have induvidual duty set.
pub struct Operator<'d, const N: u8, G: Group> {
    _instance: OPERATOR<N, G>,
    _handle: mcpwm_oper_handle_t,

    comparator_x: Comparator, // SOC_MCPWM_COMPARATORS_PER_OPERATOR is 2 for ESP32 and ESP32-S3
    comparator_y: Comparator,

    generator_a: Option<Generator<'d, G>>, // One generator per pin, with a maximum of two generators per Operator
    generator_b: Option<Generator<'d, G>>,
    //deadtime: D
}

pub(crate) unsafe fn new<const N: u8, G>(
    instance: OPERATOR<N, G>,
    timer_handle: mcpwm_timer_handle_t,
    cfg: OperatorConfig<'_>,
) -> Result<Operator<'_, N, G>, EspError>
where
    G: Group,
{
    let mut handle = ptr::null_mut();
    let mut flags: mcpwm_operator_config_t__bindgen_ty_1 = Default::default();

    // TODO: What should these be set to?
    flags.set_update_gen_action_on_tez(0);
    flags.set_update_gen_action_on_tep(0);
    flags.set_update_gen_action_on_sync(0);

    flags.set_update_dead_time_on_tez(0);
    flags.set_update_dead_time_on_tep(0);
    flags.set_update_dead_time_on_sync(0);

    let config = mcpwm_operator_config_t {
        group_id: G::ID,
        flags,
    };

    unsafe {
        esp!(esp_idf_sys::mcpwm_new_operator(&config, &mut handle))?;
    }

    let mut comparator_x = unsafe { cfg.comparator_x.init(handle)? };
    let mut comparator_y = unsafe { cfg.comparator_y.init(handle)? };

    // Connect operator to timer
    unsafe {
        esp!(mcpwm_operator_connect_timer(handle, timer_handle))?;
    }

    let generator_a = unsafe {
        cfg.generator_a
            .map(|g| g.init(handle, &mut comparator_x, &mut comparator_y))
            .transpose()?
    };
    let generator_b = unsafe {
        cfg.generator_b
            .map(|g| g.init(handle, &mut comparator_x, &mut comparator_y))
            .transpose()?
    };

    Ok(Operator {
        _instance: instance,
        _handle: handle,
        comparator_x,
        comparator_y,

        generator_a,
        generator_b,
    })
}

impl<'d, const N: u8, G> Operator<'d, N, G>
where
    G: Group,
{
    // TODO: Note that this is the comparator we are affecting, not the generator. Generator A may not necessarily have
    // anything to do with comparator A. How do we best convay that? Should we call them Generator X/Y and Comparator A/B?
    //
    // Again should we always provide two comparators? That would make both set/get_duty_a/b always available... Should we
    // instead let the user only provide only as few or many (max 2 as of ESP32/ESP32-S3) as they want. And depending on
    // expose only the corresponding set/get_duty?
    //
    // Once again to clarify, set_duty affects the comparator. The generators(booth or none) may then choose to use that
    // event, as well as timer events, to change the level of the pin.
    //
    /// Get compare value, often times same as the duty for output A.
    ///
    /// See `Self::set_compare_value_x` for more info
    pub fn get_compare_value_x(&self) -> u16 {
        todo!()
    }

    /// Set compare value, often times same as the duty for output A.
    ///
    /// Depending on how the generators are configured this is, using the most common configuration, the duty of output A.
    /// `value` is from the range 0 to timers peak value. However do note that if using a custom configuration this might
    /// control something else like for example the phase. Look into Generator::TODO for more details
    ///
    /// TODO: what about CountMode::UpDown?
    ///
    /// NOTE: The compare value shouldn’t exceed timer’s count peak, otherwise, the compare event will never got triggered.
    /// NOTE: This function is safe to from an ISR context
    #[inline(always)]
    pub fn set_compare_value_x(&mut self, value: u16) -> Result<(), EspError> {
        unsafe {
            esp!(mcpwm_comparator_set_compare_value(
                self.comparator_x.0,
                value.into()
            ))
        }
    }

    /// Get compare value, often times same as the duty for output B.
    ///
    /// See `Self::set_compare_value_x` for more info
    pub fn get_compare_value_y(&self) -> u16 {
        todo!()
    }

    /// Set compare value, often times same as the duty for output B.
    ///
    /// Depending on how the generators are configured this is, using the most common configuration, the duty of output A.
    /// `value` is from the range 0 to timers peak value. However do note that if using a custom configuration this might
    /// control something else like for example the phase. Look into Generator::TODO for more details
    ///
    /// TODO: what about CountMode::UpDown?
    ///
    /// NOTE: The compare value shouldn’t exceed timer’s count peak, otherwise, the compare event will never got triggered.
    /// NOTE: This function is safe to from an ISR context
    #[inline(always)]
    pub fn set_compare_value_y(&mut self, value: u16) -> Result<(), EspError> {
        unsafe {
            esp!(mcpwm_comparator_set_compare_value(
                self.comparator_y.0,
                value.into()
            ))
        }
    }

    /// Set force level for MCPWM generator.
    pub fn set_force_level_a(&mut self, level: Option<crate::gpio::Level>) -> Result<(), EspError> {
        let generator = self
            .generator_a
            .as_ref()
            .ok_or(EspError::from(ESP_ERR_INVALID_ARG).unwrap())?;
        let level = match level {
            None => -1,
            Some(crate::gpio::Level::High) => 1,
            Some(crate::gpio::Level::Low) => 0,
        };
        unsafe {
            esp!(esp_idf_sys::mcpwm_generator_set_force_level(
                generator.handle,
                level,
                true // TODO: Do we want support for hold_on = false?
            ))?;
        }

        Ok(())
    }

    pub fn set_force_level_b(&mut self, level: Option<crate::gpio::Level>) -> Result<(), EspError> {
        let generator = self
            .generator_b
            .as_ref()
            .ok_or(EspError::from(ESP_ERR_INVALID_ARG).unwrap())?;
        let level = match level {
            None => -1,
            Some(crate::gpio::Level::High) => 1,
            Some(crate::gpio::Level::Low) => 0,
        };
        unsafe {
            esp!(esp_idf_sys::mcpwm_generator_set_force_level(
                generator.handle,
                level,
                true // TODO: Do we want support for hold_on = false?
            ))?;
        }

        Ok(())
    }
}

pub trait OptionalOperator<const N: u8, G: Group> {}
impl<'d, const N: u8, G> OptionalOperator<N, G> for Operator<'d, N, G> where G: Group {}

pub struct NoOperator;
impl<const N: u8, G: Group> OptionalOperator<N, G> for NoOperator {}
