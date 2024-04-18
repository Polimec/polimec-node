use super::*;

#[macro_export]
/// Example:
/// ```
/// use pallet_funding::assert_close_enough;
/// use sp_arithmetic::Perquintill;
///
/// let real = 98u64;
/// let desired = 100u64;
/// assert_close_enough!(real, desired, Perquintill::from_float(0.98));
/// // This would fail
/// // assert_close_enough!(real, desired, Perquintill::from_float(0.99));
/// ```
macro_rules! assert_close_enough {
	// Match when a message is provided
	($real:expr, $desired:expr, $min_percentage:expr, $msg:expr) => {
		let actual_percentage;
		if $real <= $desired {
			actual_percentage = Perquintill::from_rational($real, $desired);
		} else {
			actual_percentage = Perquintill::from_rational($desired, $real);
		}
		assert!(actual_percentage >= $min_percentage, $msg);
	};
	// Match when no message is provided
	($real:expr, $desired:expr, $min_percentage:expr) => {
		let actual_percentage;
		if $real <= $desired {
			actual_percentage = Perquintill::from_rational($real, $desired);
		} else {
			actual_percentage = Perquintill::from_rational($desired, $real);
		}
		assert!(
			actual_percentage >= $min_percentage,
			"Actual percentage too low for the set minimum: {:?} < {:?} for {:?} and {:?}",
			actual_percentage,
			$min_percentage,
			$real,
			$desired
		);
	};
}

#[macro_export]
macro_rules! call_and_is_ok {
		($inst: expr, $( $call: expr ),* ) => {
			$inst.execute(|| {
				$(
					let result = $call;
					assert!(result.is_ok(), "Call failed: {:?}", result);
				)*
			})
		};
	}
#[macro_export]
macro_rules! find_event {
    ($runtime:ty, $pattern:pat, $($field_name:ident == $field_value:expr),+) => {
	    {
		    let events = frame_system::Pallet::<$runtime>::events();
	        events.iter().find_map(|event_record| {
			    let runtime_event = event_record.event.clone();
			    let runtime_event = <<$runtime as crate::Config>::RuntimeEvent>::from(runtime_event);
			    if let Ok(funding_event) = TryInto::<crate::Event<$runtime>>::try_into(runtime_event) {
				     if let $pattern = funding_event {
	                    let mut is_match = true;
	                    $(
	                        is_match &= $field_name == $field_value;
	                    )+
	                    if is_match {
	                        return Some(funding_event.clone());
	                    }
	                }
	                None
			    } else {
	                None
	            }
            })
	    }
    };
}

#[macro_export]
macro_rules! extract_from_event {
	($env: expr, $pattern:pat, $field:ident) => {
		$env.execute(|| {
			let events = System::events();

			events.iter().find_map(|event_record| {
				if let frame_system::EventRecord { event: RuntimeEvent::PolimecFunding($pattern), .. } = event_record {
					Some($field.clone())
				} else {
					None
				}
			})
		})
	};
}

#[macro_export]
macro_rules! define_names {
		($($name:ident: $id:expr, $label:expr);* $(;)?) => {
			$(
				pub const $name: AccountId = $id;
			)*

			pub fn names() -> std::collections::HashMap<AccountId, &'static str> {
				let mut names = std::collections::HashMap::new();
				$(
					names.insert($name, $label);
				)*
				names
			}
		};
	}
