use core::{
    alloc::Layout,
    marker::PhantomData,
    mem::{self, ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr,
};

const SMALLBOX_CAP: usize = 3;

type Storage = [usize; SMALLBOX_CAP];

pub struct SmallBox<T: ?Sized> {
    storage: MaybeUninit<Storage>,
    vptr: *const (),
    _marker: PhantomData<*mut T>,
}

struct Validate<S>(PhantomData<S>);

impl<S> Validate<S> {
    const IS_VALID: bool = {
        assert!(mem::size_of::<S>() <= mem::size_of::<Storage>());
        assert!(mem::align_of::<S>() == mem::align_of::<Storage>());
        true
    };
}

const fn has_same_layout<A: Sized, B: Sized>() -> bool {
    let lhs = Layout::new::<A>();
    let rhs = Layout::new::<B>();
    lhs.align() == rhs.align() && lhs.size() == rhs.size()
}

macro_rules! smallbox {
    ($val:expr) => {{
        let val = $val;
        let ptr = &val as *const _;
        #[allow(unsafe_code)]
        unsafe {
            $crate::smallbox::SmallBox::from_parts(val, ptr)
        }
    }};
}

pub(crate) use smallbox;

impl<T: ?Sized> SmallBox<T> {
    const IS_NON_DST_PTR: bool = has_same_layout::<*const T, *const ()>();

    const IS_FAT_PTR: bool = has_same_layout::<*const T, [*const (); 2]>();

    const IS_VALID: bool = {
        assert!(Self::IS_NON_DST_PTR || Self::IS_FAT_PTR);
        true
    };

    pub unsafe fn from_parts<S>(val: S, obj: *const T) -> Self
    where
        S: Unpin, /* + Unsize<T> */
    {
        assert!(Validate::<S>::IS_VALID);
        assert!(Self::IS_VALID);

        let mut storage: MaybeUninit<Storage> = MaybeUninit::uninit();
        ptr::write(storage.as_mut_ptr().cast::<S>(), val);

        let mut vptr = ptr::null();
        if Self::IS_FAT_PTR {
            vptr = *(&obj as *const *const T as *const *const ()).add(1);
        }

        Self {
            storage,
            vptr,
            _marker: PhantomData,
        }
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        let mut ptr: MaybeUninit<*mut T> = MaybeUninit::uninit();

        unsafe {
            let base = ptr.as_mut_ptr().cast::<*mut ()>();
            *base = self.storage.as_mut_ptr().cast::<()>();

            if Self::IS_FAT_PTR {
                *base.add(1).cast::<*const ()>() = self.vptr;
            }
        }

        unsafe { ptr.assume_init() }
    }

    pub fn as_ptr(&self) -> *const T {
        let mut ptr: MaybeUninit<*const T> = MaybeUninit::uninit();

        unsafe {
            let base = ptr.as_mut_ptr().cast::<*const ()>();
            *base = self.storage.as_ptr().cast::<()>();

            if Self::IS_FAT_PTR {
                *base.add(1).cast::<*const ()>() = self.vptr;
            }
        }

        unsafe { ptr.assume_init() }
    }

    pub fn into_inner(this: Self) -> T
    where
        T: Sized,
    {
        let mut val = ManuallyDrop::new(this);
        unsafe { ptr::read(val.as_mut_ptr()) }
    }
}

impl<T: ?Sized> Drop for SmallBox<T> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.as_mut_ptr());
        }
    }
}

impl<T: ?Sized> Deref for SmallBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_ptr() }
    }
}

impl<T: ?Sized> DerefMut for SmallBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.as_mut_ptr() }
    }
}
