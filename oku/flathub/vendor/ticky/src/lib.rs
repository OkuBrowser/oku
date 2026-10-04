/*
	This file is part of Ticky.
	Ticky is free software: you can redistribute it and/or modify
	it under the terms of the GNU Affero General Public License as published by
	the Free Software Foundation, either version 3 of the License, or
	(at your option) any later version.
	Ticky is distributed in the hope that it will be useful,
	but WITHOUT ANY WARRANTY; without even the implied warranty of
	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
	GNU Affero General Public License for more details.
	You should have received a copy of the GNU Affero General Public License
	along with Ticky.  If not, see <https://www.gnu.org/licenses/>.
*/

//! # Ticky
//! A simple stopwatch implementation.
//!
//! ## Example
//! ```rust
//! use ticky::Stopwatch;
//! let mut sw = Stopwatch::start_new();
//! // Do something …
//! sw.stop();
//! println!("Elapsed time: {}ms", sw.elapsed_ms_whole());
//! ```
//!
//! ## Features
//! - `derive_more` - Enables using [`derive_more`](https://crates.io/crates/derive_more) for deriving `From`, `Into`, `Mul`, `MulAssign`, `Div`, `DivAssign`, `Rem`, `Shr`, and `Shl` for `Stopwatch`.
//! - `hifitime` - Enables using [`hifitime`](https://crates.io/crates/hifitime) for high-resolution timekeeping.
//! - `stdtime` - Enables using [`std::time`](https://doc.rust-lang.org/std/time/index.html) for timekeeping.
//!
//! Either `hifitime` or `stdtime` must be enabled. If neither is enabled, `stdtime` is used by default. If both are enabled, `hifitime` is used.
//!
//! ## Installation
//! Run `cargo add ticky` to add Ticky to your `Cargo.toml` file.
//!
//! ## License
//! Ticky is licensed under the [GNU Affero General Public License](https://www.gnu.org/licenses/agpl-3.0.en.html).
//!
//! ## Contributing
//! Contributions are welcome! Please see [`CONTRIBUTING.md`](https://github.com/Dirout/ticky/blob/master/CONTRIBUTING.md) for more information.
//!
//! ## Authors
//! - [Emil Sayahi](https://github.com/emmyoh)
//!
//! ## Acknowledgements
//! - [`rust-stopwatch`](https://crates.io/crates/stopwatch) - The inspiration for this crate.

#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

cfg_if::cfg_if! {
	if #[cfg(any(feature = "hifitime"))] {
		use hifitime::{Duration, Epoch, TimeUnits, Unit};

		#[cfg(feature = "derive_more")]
		use derive_more::{Div, DivAssign, From, Into, Mul, MulAssign, Rem, Shl, Shr};

		#[cfg_attr(
			feature = "derive_more",
			derive(From, Into, Mul, MulAssign, Div, DivAssign, Rem, Shr, Shl,)
		)]
		#[derive(Ord, Eq, PartialEq, PartialOrd, Clone, Copy, Debug, Hash)]
		/// A simple stopwatch implementation.
		/// ## Usage
		/// ```rust
		/// use ticky::Stopwatch;
		/// let mut sw = Stopwatch::start_new();
		/// // Do something …
		/// sw.stop();
		/// println!("Elapsed time: {}ms", sw.elapsed_ms_whole());
		/// ```
		pub struct Stopwatch {
			/// The total elapsed time.
			pub elapsed: Duration,
			/// The time at which the stopwatch was last started.
			pub timer: Duration,
			/// Whether the stopwatch is currently running.
			pub is_running: bool,
		}

		impl Display for Stopwatch {
			fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
				write!(f, "{}", self.elapsed())
			}
		}

		impl From<Duration> for Stopwatch {
			fn from(dur: Duration) -> Self {
				Self {
					elapsed: dur,
					timer: Epoch::now().unwrap().to_duration(),
					is_running: false,
				}
			}
		}

		impl Default for Stopwatch {
			fn default() -> Self {
				Self {
					elapsed: Duration::ZERO,
					timer: Epoch::now().unwrap().to_duration(),
					is_running: false,
				}
			}
		}

		impl Stopwatch {
			/// Starts (or resumes) the stopwatch.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch (pausing it)
			/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 100); // Allow for some error (± 100 milliseconds)
			/// sw.start(); // Start the stopwatch again (resuming it)
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed_ms_whole().abs_diff(2_000) < 100); // Allow for some error (± 100 milliseconds)
			/// ```
			pub fn start(&mut self) {
				self.timer = Epoch::now().unwrap().to_duration();
				self.is_running = true;
			}

			/// Stops (or pauses) the stopwatch.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch (pausing it)
			/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 100); // Allow for some error (± 100 milliseconds)
			/// sw.start(); // Start the stopwatch again (resuming it)
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed_ms_whole().abs_diff(2_000) < 100); // Allow for some error (± 100 milliseconds)
			/// ```
			pub fn stop(&mut self) {
				self.elapsed += (Epoch::now().unwrap().to_duration() - self.timer).abs();
				self.is_running = false;
			}

			/// Returns the total elapsed time.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!((sw.elapsed().to_unit(hifitime::prelude::Unit::Millisecond) - 1_000.0).abs() < 100.0); // Allow for some error (± 100 milliseconds)
			/// ```
			pub fn elapsed(&self) -> Duration {
				match self.is_running {
					true =>  self.elapsed + (Epoch::now().unwrap().to_duration() - self.timer).abs(),
					false => self.elapsed,
				}
			}

			/// Returns the total elapsed time in fractional milliseconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!((sw.elapsed_ms() - 1_000.0).abs() < 100.0); // Allow for some error (± 100 milliseconds)
			/// ```
			pub fn elapsed_ms(&mut self) -> f64 {
				self.elapsed().to_unit(Unit::Millisecond)
			}

			/// Returns the total elapsed time in whole milliseconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 100); // Allow for some error (± 100 milliseconds)
			/// ```
			pub fn elapsed_ms_whole(&mut self) -> u128 {
				self.elapsed().round(1.milliseconds()).to_unit(Unit::Millisecond).round() as u128
			}

			/// Returns the total elapsed time in fractional microseconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_micros(1_000_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!((sw.elapsed_us() - 1_000_000.0).abs() < 10_000.0); // Allow for some error (± 10,000 microseconds)
			/// ```
			pub fn elapsed_us(&mut self) -> f64 {
				self.elapsed().to_unit(Unit::Microsecond)
			}

			/// Returns the total elapsed time in whole microseconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_micros(1_000_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed_us_whole().abs_diff(1_000_000) < 10_000); // Allow for some error (± 10,000 microseconds)
			/// ```
			pub fn elapsed_us_whole(&mut self) -> u128 {
				self.elapsed().round(1.microseconds()).to_unit(Unit::Microsecond).round() as u128
			}

			/// Returns the total elapsed time in fractional nanoseconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_nanos(1_000_000_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!((sw.elapsed_ns() - 1_000_000_000.0).abs() < 100_000_000.0); // Allow for some error (± 100,000,000 nanoseconds)
			/// ```
			pub fn elapsed_ns(&mut self) -> f64 {
				self.elapsed().to_unit(Unit::Nanosecond)
			}

			/// Returns the total elapsed time in whole nanoseconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_nanos(1_000_000_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed_ns_whole().abs_diff(1_000_000_000) < 100_000_000); // Allow for some error (± 100,000,000 nanoseconds)
			/// ```
			pub fn elapsed_ns_whole(&mut self) -> u128 {
				self.elapsed().total_nanoseconds().try_into().unwrap()
			}

			/// Returns the total elapsed time in fractional seconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_500)); // Wait for 1.5 seconds
			/// sw.stop(); // Stop the stopwatch
			/// assert!((sw.elapsed_s() - 1.5).abs() < 0.01); // Allow for some error (± 0.01 seconds)
			/// ```
			pub fn elapsed_s(&mut self) -> f64 {
				self.elapsed().to_unit(Unit::Second)
			}

			/// Returns the total elapsed time in whole seconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_500)); // Wait for 1.5 seconds
			/// sw.stop(); // Stop the stopwatch
			/// assert_eq!(sw.elapsed_s_whole(), 2);
			/// ```
			pub fn elapsed_s_whole(&self) -> u64 {
				self.elapsed().round(1.seconds()).to_seconds().round() as u64
			}
		}

	} else if #[cfg(feature = "stdtime")] {
		use core::time::Duration;

		#[cfg(feature = "std")]
		use std::time::Instant;

		#[cfg(feature = "derive_more")]
		use derive_more::{Div, DivAssign, From, Into, Mul, MulAssign, Rem, Shl, Shr};

		#[cfg_attr(
			feature = "derive_more",
			derive(From, Into, Mul, MulAssign, Div, DivAssign, Rem, Shr, Shl,)
		)]
		#[derive(Ord, Eq, PartialEq, PartialOrd, Clone, Copy, Debug, Hash)]
		/// A simple stopwatch implementation.
		/// ## Usage
		/// ```rust
		/// use ticky::Stopwatch;
		/// let mut sw = Stopwatch::start_new();
		/// // Do something …
		/// sw.stop();
		/// println!("Elapsed time: {}ms", sw.elapsed_ms_whole());
		/// ```
		pub struct Stopwatch {
			/// The total elapsed time.
			pub elapsed: Duration,
			/// The time at which the stopwatch was last started.
			pub timer: Instant,
			/// Whether the stopwatch is currently running.
			pub is_running: bool,
		}

		impl Display for Stopwatch {
			fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
				let elapsed_ms = self.clone().elapsed_ms_whole();
				write!(f, "{elapsed_ms}ms")
			}
		}

		impl From<Duration> for Stopwatch {
			fn from(dur: Duration) -> Self {
				Self {
					elapsed: dur,
					timer: Instant::now(),
					is_running: false,
				}
			}
		}

		impl Default for Stopwatch {
			fn default() -> Self {
				Self {
					elapsed: Duration::new(0, 0),
					timer: Instant::now(),
					is_running: false,
				}
			}
		}

		impl Stopwatch {
			/// Starts (or resumes) the stopwatch.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch (pausing it)
			/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 100); // Allow for some error (± 100 milliseconds)
			/// sw.start(); // Start the stopwatch again (resuming it)
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed_ms_whole().abs_diff(2_000) < 100); // Allow for some error (± 100 milliseconds)
			/// ```
			pub fn start(&mut self) {
				self.timer = Instant::now();
				self.is_running = true;
			}

			/// Stops (or pauses) the stopwatch.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch (pausing it)
			/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 100); // Allow for some error (± 100 milliseconds)
			/// sw.start(); // Start the stopwatch again (resuming it)
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed_ms_whole().abs_diff(2_000) < 100); // Allow for some error (± 100 milliseconds)
			/// ```
			pub fn stop(&mut self) {
				self.elapsed += self.timer.elapsed();
				self.is_running = false;
			}

			/// Returns the total elapsed time.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed().as_millis().abs_diff(1_000) < 100); // Allow for some error (± 100 milliseconds)
			/// ```
			pub fn elapsed(&self) -> Duration {
				match self.is_running {
					true => self.elapsed + self.timer.elapsed(),
					false => self.elapsed,
				}
			}

			/// Returns the total elapsed time in milliseconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 100); // Allow for some error (± 100 milliseconds)
			/// ```
			pub fn elapsed_ms_whole(&mut self) -> u128 {
				self.elapsed().as_millis()
			}

			/// Returns the total elapsed time in microseconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_micros(1_000_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed_us_whole().abs_diff(1_000_000) < 10_000); // Allow for some error (± 10,000 microseconds)
			/// ```
			pub fn elapsed_us_whole(&mut self) -> u128 {
				self.elapsed().as_micros()
			}

			/// Returns the total elapsed time in nanoseconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_nanos(1_000_000_000)); // Wait for 1 second
			/// sw.stop(); // Stop the stopwatch
			/// assert!(sw.elapsed_ns_whole().abs_diff(1_000_000_000) < 100_000_000); // Allow for some error (± 100,000,000 nanoseconds)
			/// ```
			pub fn elapsed_ns_whole(&mut self) -> u128 {
				self.elapsed().as_nanos()
			}

			/// Returns the total elapsed time in fractional seconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_500)); // Wait for 1.5 seconds
			/// sw.stop(); // Stop the stopwatch
			/// assert!((sw.elapsed_s() - 1.5).abs() < 0.01); // Allow for some error (± 0.01 seconds)
			/// ```
			pub fn elapsed_s(&mut self) -> f64 {
				self.elapsed().as_secs_f64()
			}

			/// Returns the total elapsed time in whole seconds.
			///
			/// # Example
			/// ```rust
			/// use ticky::Stopwatch;
			///
			/// let mut sw = Stopwatch::new(); // Create a new stopwatch
			/// sw.start(); // Start the stopwatch
			/// std::thread::sleep(std::time::Duration::from_millis(1_500)); // Wait for 1.5 seconds
			/// sw.stop(); // Stop the stopwatch
			/// assert_eq!(sw.elapsed_s_whole(), 1);
			/// ```
			pub fn elapsed_s_whole(&self) -> u64 {
				self.elapsed().as_secs()
			}
		}
	} else {
		compile_error!("You must enable either the `stdtime` or `hifitime` feature.");
	}
}

use core::fmt::{Display, Formatter};

impl From<Stopwatch> for Duration {
	fn from(sw: Stopwatch) -> Self {
		sw.elapsed()
	}
}

impl Stopwatch {
	/// Creates a new stopwatch.
	///
	/// # Example
	/// ```rust
	/// use ticky::Stopwatch;
	///
	/// let mut sw = Stopwatch::new(); // Create a new stopwatch
	/// sw.start(); // Start the stopwatch
	/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
	/// sw.stop(); // Stop the stopwatch
	/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 10); // Allow for some error (± 10 milliseconds)
	/// ```
	pub fn new() -> Stopwatch {
		Stopwatch::default()
	}

	/// Creates a new stopwatch and starts it.
	///
	/// # Example
	/// ```rust
	/// use ticky::Stopwatch;
	///
	/// let mut sw = Stopwatch::start_new(); // Create a new stopwatch, and start it
	/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
	/// sw.stop(); // Stop the stopwatch
	/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 10); // Allow for some error (± 10 milliseconds)
	/// ```
	pub fn start_new() -> Stopwatch {
		let mut sw = Stopwatch::new();
		sw.start();
		sw
	}

	/// Resets the stopwatch.
	///
	/// # Example
	/// ```rust
	/// use ticky::Stopwatch;
	///
	/// let mut sw = Stopwatch::new(); // Create a new stopwatch
	/// sw.start(); // Start the stopwatch
	/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
	/// sw.stop(); // Stop the stopwatch
	/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 10); // Allow for some error (± 10 milliseconds)
	/// sw.reset(); // Reset the stopwatch
	/// assert!(sw.elapsed_ms_whole().abs_diff(0) < 10); // Allow for some error (± 10 milliseconds)
	/// sw.start(); // Start the stopwatch
	/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
	/// sw.stop(); // Stop the stopwatch
	/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 10); // Allow for some error (± 10 milliseconds)
	pub fn reset(&mut self) {
		*self = Stopwatch::new();
	}

	/// Resets and starts the stopwatch.
	///
	/// # Example
	/// ```rust
	/// use ticky::Stopwatch;
	///
	/// let mut sw = Stopwatch::new(); // Create a new stopwatch
	/// sw.start(); // Start the stopwatch
	/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
	/// sw.stop(); // Stop the stopwatch
	/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 10); // Allow for some error (± 10 milliseconds)
	/// sw.restart(); // Reset and start the stopwatch
	/// std::thread::sleep(std::time::Duration::from_millis(1_000)); // Wait for 1 second
	/// sw.stop(); // Stop the stopwatch
	/// assert!(sw.elapsed_ms_whole().abs_diff(1_000) < 10); // Allow for some error (± 10 milliseconds)
	pub fn restart(&mut self) {
		self.reset();
		self.start();
	}

	/// Returns true if the stopwatch is running, and false if not.
	///
	/// # Example
	/// ```rust
	/// use ticky::Stopwatch;
	///
	/// let mut sw = Stopwatch::new(); // Create a new stopwatch
	/// assert_eq!(sw.is_running(), false); // The stopwatch is not running
	/// sw.start(); // Start the stopwatch
	/// assert_eq!(sw.is_running(), true); // The stopwatch is running
	/// sw.stop(); // Stop the stopwatch
	/// assert_eq!(sw.is_running(), false); // The stopwatch is not running
	/// ```
	pub fn is_running(&mut self) -> bool {
		self.is_running
	}
}
