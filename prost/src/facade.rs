//! A facade around all the types we need from the `std`, `core`, and `alloc`
//! crates. This avoids elaborate import wrangling having to happen in every
//! module.

mod core {
    #[cfg(not(feature = "std"))]
    pub use core::*;
    #[cfg(feature = "std")]
    pub use std::*;
}

#[cfg(feature = "std")]
pub use self::core::{cmp, mem, slice};

pub use self::core::ascii;
pub use self::core::borrow::Borrow;
pub use self::core::cell::{Cell, RefCell};
pub use self::core::clone;
pub use self::core::cmp::min;
pub use self::core::cmp::Reverse;
pub use self::core::collections::hash_map;
pub use self::core::convert;
pub use self::core::default;
pub use self::core::env;
pub use self::core::fmt::{self, Debug, Display, Write as FmtWrite};
pub use self::core::format;
pub use self::core::marker::{self, PhantomData};
pub use self::core::num::Wrapping;
pub use self::core::ops::RangeToInclusive;
pub use self::core::ops::{Bound, Range, RangeFrom, RangeInclusive, RangeTo};
pub use self::core::option;
pub use self::core::result;
pub use self::core::str::FromStr;
pub use self::core::time;
pub use self::core::{f32, f64};
pub use self::core::{io, io::Read};
pub use self::core::{iter, num, ptr, str};

#[cfg(feature = "std")]
pub use std::rc::Rc;
#[cfg(feature = "std")]
pub use std::{fs, fs::File};

#[cfg(not(feature = "std"))]
pub use alloc::borrow::{Cow, ToOwned};
#[cfg(feature = "std")]
pub use std::borrow::{Cow, ToOwned};

#[cfg(not(feature = "std"))]
pub use alloc::string::{String, ToString};
#[cfg(feature = "std")]
pub use std::string::{String, ToString};

#[cfg(not(feature = "std"))]
pub use alloc::vec::Vec;
#[cfg(feature = "std")]
pub use std::vec::Vec;

#[cfg(not(feature = "std"))]
pub use alloc::vec;
#[cfg(feature = "std")]
pub use std::vec;

#[cfg(not(feature = "std"))]
pub use alloc::boxed::Box;
#[cfg(feature = "std")]
pub use std::boxed::Box;

#[cfg(not(feature = "std"))]
pub use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};
#[cfg(feature = "std")]
pub use std::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};

#[cfg(feature = "std")]
pub use std::{error, net};

#[cfg(feature = "std")]
pub use std::collections::{HashMap, HashSet};
#[cfg(feature = "std")]
pub use std::ffi::{OsStr, OsString};
#[cfg(feature = "std")]
pub use std::hash::{BuildHasher, Hash};
#[cfg(feature = "std")]
pub use std::io::Write;
#[cfg(feature = "std")]
pub use std::path::{Path, PathBuf};
#[cfg(feature = "std")]
pub use std::sync::{Mutex, RwLock};
#[cfg(feature = "std")]
pub use std::time::{SystemTime, UNIX_EPOCH};
