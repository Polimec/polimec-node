// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// The Polimec Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Polimec Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#[macro_export]
macro_rules! _actual_percentage {
	($real:expr, $desired:expr) => {{
		let r_val = $real;
		let d_val = $desired;
		if r_val == 0 && d_val == 0 {
			Perquintill::one()
		} else if r_val <= d_val {
			Perquintill::from_rational(r_val, d_val)
		} else {
			Perquintill::from_rational(d_val, r_val)
		}
	}};
}

#[macro_export]
/// Example:
/// ```
/// use pallet_funding::assert_close_enough;
/// use sp_runtime::Perquintill;
///
/// let real = 98u64;
/// let desired = 100u64;
/// assert_close_enough!(real, desired, Perquintill::from_float(0.98));
/// // This would fail:
/// // assert_close_enough!(real, desired, Perquintill::from_float(0.99));
///
/// assert_close_enough!(0u64, 0u64, Perquintill::one()); // 0 is 100% close to 0
/// assert_close_enough!(real, desired, Perquintill::from_float(0.98), "Custom message: values differ too much");
/// ```
macro_rules! assert_close_enough {
    // Match when a custom message is provided
    ($real:expr, $desired:expr, $min_percentage:expr, $($msg:tt)+) => {
        {
            let min_p_val = $min_percentage;
            let actual_percentage = $crate::_actual_percentage!($real, $desired);
            assert!(actual_percentage >= min_p_val, $($msg)+);
        }
    };
    // Match when no custom message is provided
    ($real:expr, $desired:expr, $min_percentage:expr) => {
        {
            let min_p_val = $min_percentage;
            let actual_percentage = $crate::_actual_percentage!($real, $desired);
            assert!(
                actual_percentage >= min_p_val,
                "Actual percentage ({:?}) is less than the required minimum ({:?}) for values {:?} and {:?}",
                actual_percentage,
                min_p_val,
                $real,
                $desired
            );
        }
    };
}

#[macro_export]
/// Checks if two values are close enough based on a minimum percentage.
///
/// Example:
/// ```
/// use pallet_funding::is_close_enough;
/// use sp_runtime::Perquintill;
///
/// let real = 98u64;
/// let desired = 100u64;
/// assert!(is_close_enough!(real, desired, Perquintill::from_float(0.98)));
/// assert!(!is_close_enough!(real, desired, Perquintill::from_float(0.99)));
/// assert!(is_close_enough!(0u64, 0u64, Perquintill::one()));
/// ```
macro_rules! is_close_enough {
	($real:expr, $desired:expr, $min_percentage:expr) => {{
		let r_val = $real;
		let d_val = $desired;
		let min_p_val = $min_percentage;

		let actual_percentage = if r_val == 0 && d_val == 0 {
			Perquintill::one()
		} else if r_val <= d_val {
			// If d_val is 0 here, r_val must also be 0 (covered above). So d_val > 0.
			Perquintill::from_rational(r_val, d_val)
		} else {
			// r_val > d_val
			// If r_val is 0 here, d_val must also be 0 (covered above). So r_val > 0.
			Perquintill::from_rational(d_val, r_val)
		};
		actual_percentage >= min_p_val
	}};
}

#[macro_export]
/// Finds a specific pallet event in the system events.
///
/// - `$runtime`: The runtime type (e.g., `TestRuntime`).
/// - `$pattern`: The pattern to match against the pallet's event (e.g., `Event::MyEvent { field, .. }`).
///   `Event` in the pattern must be in scope and refer to the pallet's event enum.
/// - `$($field_name:ident == $field_value:expr),+`: One or more conditions to check on the
///   fields bound by the pattern.
///
/// Example (assuming used within the pallet's tests where `crate::Event` is the pallet's event enum):
/// ```rust,ignore
/// use my_runtime::Runtime; // Replace with your actual runtime
/// use crate::Event;        // Ensure your pallet's Event enum is in scope
///
/// // Mock System events for an example
/// fn mock_system_events(event: RuntimeEvent) {
///     frame_system::Pallet::<Runtime>::deposit_event(event.into());
/// }
///
/// let alice_account_id = sp_core::sr25519::Public::from_raw([0u8; 32]);
/// // Suppose you have an event: crate::Event::SomethingHappened { who: AccountId, amount: u128 }
/// // mock_system_events(crate::Event::SomethingHappened { who: alice_account_id, amount: 100 }.into());
///
/// let found_event = find_event!(
///     Runtime,
///     Event::SomethingHappened { who, amount, .. }, // `Event` here refers to `crate::Event`
///     who == alice_account_id,
///     amount == 100
/// );
/// assert!(found_event.is_some());
/// ```
macro_rules! find_event {
    // Case with field checks
    ($runtime:ty, $pattern:pat, $($field_name:ident == $field_value:expr),+) => {
        {
            // Get all system events for the current block.
            // `event_record.event` is of type `<<$runtime as frame_system::Config>::RuntimeEvent>`
            let events = frame_system::Pallet::<$runtime>::events();

            events.iter().find_map(|event_record| {
                // `event_record.event` is the "outer" runtime event.
                // We need to try and convert it into our *specific pallet's* event.
                // The type `Event<$runtime>` is assumed to be the pallet's event enum,
                // (e.g., `crate::Event<$runtime>` or `your_pallet::Event<$runtime>`).
                // This relies on the necessary `TryFrom` or `TryInto` implementation being available.
                if let Ok(pallet_event_instance) = <_ as TryInto<Event<$runtime>>>::try_into(event_record.event.clone()) {
                    // `pallet_event_instance` is now of type `Event<$runtime>` (the pallet's event).

                    // Clone the successfully converted pallet event *before* it's potentially
                    // moved by the pattern matching. This clone is what we'll return.
                    let event_candidate_for_return = pallet_event_instance.clone();

                    // Match the specific pallet event against the provided pattern.
                    // This will bind variables like `who`, `amount` from the pattern.
                    if let $pattern = pallet_event_instance { // `pallet_event_instance` is moved here
                        let mut all_fields_match = true;
                        // Check each field condition using the variables bound by `$pattern`.
                        $(
                            all_fields_match &= ($field_name == $field_value);
                        )+

                        if all_fields_match {
                            return Some(event_candidate_for_return); // Return the clone
                        }
                    }
                }
                // If TryInto failed, or pattern didn't match, or field checks failed, continue to next event.
                None
            })
        }
    };
    // Case without field checks (pattern only) - useful if the pattern itself is sufficiently specific.
    ($runtime:ty, $pattern:pat) => {
        {
            let events = frame_system::Pallet::<$runtime>::events();
            events.iter().find_map(|event_record| {
                if let Ok(pallet_event_instance) = <_ as TryInto<Event<$runtime>>>::try_into(event_record.event.clone()) {
                    let event_candidate_for_return = pallet_event_instance.clone();
                    if let $pattern = pallet_event_instance {
                        return Some(event_candidate_for_return);
                    }
                }
                None
            })
        }
    };
}
