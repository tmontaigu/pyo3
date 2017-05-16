// Copyright (c) 2016 Daniel Grunwald
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this
// software and associated documentation files (the "Software"), to deal in the Software
// without restriction, including without limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons
// to whom the Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or
// substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
// INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
// PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
// OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use ffi;
use std::{isize, ptr};
use std::os::raw::{c_int};
use python::{Python, PythonObject};
use conversion::ToPyObject;
use function::CallbackConverter;
use err::{PyErr, PyResult};
use py_class::{CompareOp};
use exc;
use Py_hash_t;

#[macro_export]
#[doc(hidden)]
macro_rules! py_class_type_object_static_init {
    ($class_name:ident,
     $gc:tt,
    /* slots: */ {
        /* type_slots */  [ $( $slot_name:ident : $slot_value:expr, )* ]
        $as_async:tt
        $as_number:tt
        $as_sequence:tt
        $as_mapping:tt
        $as_buffer:tt
        $setdelitem:tt
    }) => (
        $crate::_detail::ffi::PyTypeObject {
            $( $slot_name : $slot_value, )*
            ..
                $crate::_detail::ffi::PyTypeObject_INIT
        }
    );
}

#[macro_export]
#[doc(hidden)]
macro_rules! py_class_wrap_newfunc {
    ($class:ident :: $f:ident [ $( { $pname:ident : $ptype:ty = $detail:tt } )* ]) => {{
        unsafe extern "C" fn wrap_newfunc(
            cls: *mut $crate::_detail::ffi::PyTypeObject,
            args: *mut $crate::_detail::ffi::PyObject,
            kwargs: *mut $crate::_detail::ffi::PyObject)
        -> *mut $crate::_detail::ffi::PyObject
        {
            const LOCATION: &'static str = concat!(stringify!($class), ".", stringify!($f), "()");
            $crate::_detail::handle_callback(
                LOCATION, $crate::_detail::PyObjectCallbackConverter,
                |py| {
                    py_argparse_raw!(py, Some(LOCATION), args, kwargs,
                        [ $( { $pname : $ptype = $detail } )* ]
                        {
                            let cls = $crate::PyType::from_type_ptr(py, cls);
                            let ret = $class::$f(&cls, py $(, $pname )* );
                            $crate::PyDrop::release_ref(cls, py);
                            ret
                        })
                })
        }
        Some(wrap_newfunc)
    }}
}

#[macro_export]
#[doc(hidden)]
macro_rules! py_class_as_number {
    ([]) => (0 as *mut $crate::_detail::ffi::PyNumberMethods);
    ([$( $slot_name:ident : $slot_value:expr ,)+]) => {{
        static mut NUMBER_METHODS : $crate::_detail::ffi::PyNumberMethods
            = $crate::_detail::ffi::PyNumberMethods {
                $( $slot_name : $slot_value, )*
                ..
                    $crate::_detail::ffi::PyNumberMethods_INIT
            };
        unsafe { &mut NUMBER_METHODS }
    }}
}

#[macro_export]
#[doc(hidden)]
macro_rules! py_class_unary_slot {
    ($class:ident :: $f:ident, $res_type:ty, $conv:expr) => {{
        unsafe extern "C" fn wrap_unary(
            slf: *mut $crate::_detail::ffi::PyObject)
        -> $res_type
        {
            const LOCATION: &'static str = concat!(stringify!($class), ".", stringify!($f), "()");
            $crate::_detail::handle_callback(
                LOCATION, $conv,
                |py| {
                    let slf = $crate::PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<$class>();
                    let ret = slf.$f(py);
                    $crate::PyDrop::release_ref(slf, py);
                    ret
                })
        }
        Some(wrap_unary)
    }}
}

#[macro_export]
#[doc(hidden)]
macro_rules! py_class_binary_slot {
    ($class:ident :: $f:ident, $arg_type:ty, $res_type:ty, $conv:expr) => {{
        unsafe extern "C" fn wrap_binary(
            slf: *mut $crate::_detail::ffi::PyObject,
            arg: *mut $crate::_detail::ffi::PyObject)
        -> $res_type
        {
            const LOCATION: &'static str = concat!(stringify!($class), ".", stringify!($f), "()");
            $crate::_detail::handle_callback(
                LOCATION, $conv,
                |py| {
                    let slf = $crate::PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<$class>();
                    let arg = $crate::PyObject::from_borrowed_ptr(py, arg);
                    let ret = match <$arg_type as $crate::FromPyObject>::extract(py, &arg) {
                        Ok(arg) => slf.$f(py, arg),
                        Err(e) => Err(e)
                    };
                    $crate::PyDrop::release_ref(arg, py);
                    $crate::PyDrop::release_ref(slf, py);
                    ret
                })
        }
        Some(wrap_binary)
    }}
}

#[macro_export]
#[doc(hidden)]
macro_rules! py_class_ternary_slot {
    ($class:ident :: $f:ident, $arg1_type:ty, $arg2_type:ty, $res_type:ty, $conv:expr) => {{
        unsafe extern "C" fn wrap_binary(
            slf: *mut $crate::_detail::ffi::PyObject,
            arg1: *mut $crate::_detail::ffi::PyObject,
            arg2: *mut $crate::_detail::ffi::PyObject)
        -> $res_type
        {
            const LOCATION: &'static str = concat!(stringify!($class), ".", stringify!($f), "()");
            $crate::_detail::handle_callback(
                LOCATION, $conv,
                |py| {
                    let slf = $crate::PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<$class>();
                    let arg1 = $crate::PyObject::from_borrowed_ptr(py, arg1);
                    let arg2 = $crate::PyObject::from_borrowed_ptr(py, arg2);
                    let ret = match <$arg1_type as $crate::FromPyObject>::extract(py, &arg1) {
                        Ok(arg1) => match <$arg2_type as $crate::FromPyObject>::extract(py, &arg2) {
                            Ok(arg2) => slf.$f(py, arg1, arg2),
                            Err(e) => Err(e)
                        },
                        Err(e) => Err(e)
                    };
                    $crate::PyDrop::release_ref(arg1, py);
                    $crate::PyDrop::release_ref(arg2, py);
                    $crate::PyDrop::release_ref(slf, py);
                    ret
                })
        }
        Some(wrap_binary)
    }}
}


#[macro_export]
#[doc(hidden)]
macro_rules! py_class_binary_internal {
    ($class:ident :: $f:ident, $arg1_type:ty, $res_type:ty, $conv:expr) => {{
        unsafe extern "C" fn wrap_binary(
            slf: *mut $crate::_detail::ffi::PyObject,
            arg1: $arg1_type)
            -> $res_type                                                          {
            const LOCATION: &'static str = concat!(stringify!($class), ".", stringify!($f), "()");
            $crate::_detail::handle_callback(
                LOCATION, $conv,
                |py| {
                    let slf = $crate::PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<$class>();
                    Ok(slf.$f(py, arg1))
                })
        }
        Some(wrap_binary)
    }}
}


#[macro_export]
#[doc(hidden)]
macro_rules! py_class_ternary_internal {
    ($class:ident :: $f:ident, $arg1_type:ty, $arg2_type:ty, $res_type:ty, $conv:expr) => {{
        unsafe extern "C" fn wrap(
            slf: *mut $crate::_detail::ffi::PyObject,
            arg1: $arg1_type,
            arg2: $arg2_type)
            -> $res_type
        {
            const LOCATION: &'static str = concat!(stringify!($class), ".", stringify!($f), "()");
            $crate::_detail::handle_callback(
                LOCATION, $conv,
                |py| {
                    let slf = $crate::PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<$class>();
                    Ok(slf.$f(py, arg1, arg2))
                })
        }
        Some(wrap)
    }}
}


pub fn extract_op(py: Python, op: c_int) -> PyResult<CompareOp> {
    match op {
        ffi::Py_LT => Ok(CompareOp::Lt),
        ffi::Py_LE => Ok(CompareOp::Le),
        ffi::Py_EQ => Ok(CompareOp::Eq),
        ffi::Py_NE => Ok(CompareOp::Ne),
        ffi::Py_GT => Ok(CompareOp::Gt),
        ffi::Py_GE => Ok(CompareOp::Ge),
        _ => Err(PyErr::new_lazy_init(
            py.get_type::<exc::ValueError>(),
            Some("tp_richcompare called with invalid comparison operator".to_py_object(py).into_object())))
    }
}

// sq_richcompare is special-cased slot
#[macro_export]
#[doc(hidden)]
macro_rules! py_class_richcompare_slot {
    ($class:ident :: $f:ident, $arg_type:ty, $res_type:ty, $conv:expr) => {{
        unsafe extern "C" fn tp_richcompare(
            slf: *mut $crate::_detail::ffi::PyObject,
            arg: *mut $crate::_detail::ffi::PyObject,
            op: $crate::_detail::libc::c_int)
        -> $res_type
        {
            const LOCATION: &'static str = concat!(stringify!($class), ".", stringify!($f), "()");
            $crate::_detail::handle_callback(
                LOCATION, $conv,
                |py| {
                    let slf = $crate::PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<$class>();
                    let arg = $crate::PyObject::from_borrowed_ptr(py, arg);
                    let ret = match $crate::py_class::slots::extract_op(py, op) {
                        Ok(op) => match <$arg_type as $crate::FromPyObject>::extract(py, &arg) {
                            Ok(arg) => slf.$f(py, arg, op).map(|res| { res.into_py_object(py).into_object() }),
                            Err(_) => Ok(py.NotImplemented())
                        },
                        Err(_) => Ok(py.NotImplemented())
                    };
                    $crate::PyDrop::release_ref(arg, py);
                    $crate::PyDrop::release_ref(slf, py);
                    ret
                })
        }
        Some(tp_richcompare)
    }}
}

// sq_contains is special-cased slot because it converts type errors to Ok(false)
#[macro_export]
#[doc(hidden)]
macro_rules! py_class_contains_slot {
    ($class:ident :: $f:ident, $arg_type:ty) => {{
        unsafe extern "C" fn sq_contains(
            slf: *mut $crate::_detail::ffi::PyObject,
            arg: *mut $crate::_detail::ffi::PyObject)
        -> $crate::_detail::libc::c_int
        {
            const LOCATION: &'static str = concat!(stringify!($class), ".", stringify!($f), "()");
            $crate::_detail::handle_callback(
                LOCATION, $crate::py_class::slots::BoolConverter,
                |py| {
                    let slf = $crate::PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<$class>();
                    let arg = $crate::PyObject::from_borrowed_ptr(py, arg);
                    let ret = match <$arg_type as $crate::FromPyObject>::extract(py, &arg) {
                        Ok(arg) => slf.$f(py, arg),
                        Err(e) => $crate::py_class::slots::type_error_to_false(py, e)
                    };
                    $crate::PyDrop::release_ref(arg, py);
                    $crate::PyDrop::release_ref(slf, py);
                    ret
                })
        }
        Some(sq_contains)
    }}
}

pub fn type_error_to_false(py: Python, e: PyErr) -> PyResult<bool> {
    if e.matches(py, py.get_type::<exc::TypeError>()) {
        Ok(false)
    } else {
        Err(e)
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! py_class_binary_numeric_slot {
    ($class:ident :: $f:ident) => {{
        unsafe extern "C" fn binary_numeric(
            lhs: *mut $crate::_detail::ffi::PyObject,
            rhs: *mut $crate::_detail::ffi::PyObject)
        -> *mut $crate::_detail::ffi::PyObject
        {
            const LOCATION: &'static str = concat!(stringify!($class), ".", stringify!($f), "()");
            $crate::_detail::handle_callback(
                LOCATION, $crate::_detail::PyObjectCallbackConverter,
                |py| {
                    let lhs = $crate::PyObject::from_borrowed_ptr(py, lhs);
                    let rhs = $crate::PyObject::from_borrowed_ptr(py, rhs);
                    let ret = $class::$f(py, &lhs, &rhs);
                    $crate::PyDrop::release_ref(lhs, py);
                    $crate::PyDrop::release_ref(rhs, py);
                    ret
                })
        }
        Some(binary_numeric)
    }}
}


pub struct VoidCallbackConverter;

impl CallbackConverter<()> for VoidCallbackConverter {
    type R = ();

    #[inline]
    fn convert(_: (), _: Python) -> () {
        ()
    }

    #[inline]
    fn error_value() -> () {
        ()
    }
}


pub struct UnitCallbackConverter;

impl CallbackConverter<()> for UnitCallbackConverter {
    type R = c_int;

    #[inline]
    fn convert(_: (), _: Python) -> c_int {
        0
    }

    #[inline]
    fn error_value() -> c_int {
        -1
    }
}

pub struct LenResultConverter;

impl CallbackConverter<usize> for LenResultConverter {
    type R = isize;

    fn convert(val: usize, py: Python) -> isize {
        if val <= (isize::MAX as usize) {
            val as isize
        } else {
            PyErr::new_lazy_init(py.get_type::<exc::OverflowError>(), None).restore(py);
            -1
        }
    }

    #[inline]
    fn error_value() -> isize {
        -1
    }
}

pub struct IterNextResultConverter;

impl <T> CallbackConverter<Option<T>>
    for IterNextResultConverter
    where T: ToPyObject
{
    type R = *mut ffi::PyObject;

    fn convert(val: Option<T>, py: Python) -> *mut ffi::PyObject {
        match val {
            Some(val) => val.into_py_object(py).into_object().steal_ptr(),
            None => unsafe {
                ffi::PyErr_SetNone(ffi::PyExc_StopIteration);
                ptr::null_mut()
            }
        }
    }

    #[inline]
    fn error_value() -> *mut ffi::PyObject {
        ptr::null_mut()
    }
}

pub trait WrappingCastTo<T> {
    fn wrapping_cast(self) -> T;
}

macro_rules! wrapping_cast {
    ($from:ty, $to:ty) => {
        impl WrappingCastTo<$to> for $from {
            #[inline]
            fn wrapping_cast(self) -> $to {
                self as $to
            }
        }
    }
}
wrapping_cast!(u8, Py_hash_t);
wrapping_cast!(u16, Py_hash_t);
wrapping_cast!(u32, Py_hash_t);
wrapping_cast!(usize, Py_hash_t);
wrapping_cast!(u64, Py_hash_t);
wrapping_cast!(i8, Py_hash_t);
wrapping_cast!(i16, Py_hash_t);
wrapping_cast!(i32, Py_hash_t);
wrapping_cast!(isize, Py_hash_t);
wrapping_cast!(i64, Py_hash_t);

pub struct HashConverter;

impl <T> CallbackConverter<T> for HashConverter
    where T: WrappingCastTo<Py_hash_t>
{
    type R = Py_hash_t;

    #[inline]
    fn convert(val: T, _py: Python) -> Py_hash_t {
        let hash = val.wrapping_cast();
        if hash == -1 {
            -2
        } else {
            hash
        }
    }

    #[inline]
    fn error_value() -> Py_hash_t {
        -1
    }
}

pub struct BoolConverter;

impl CallbackConverter<bool> for BoolConverter {
    type R = c_int;

    #[inline]
    fn convert(val: bool, _py: Python) -> c_int {
        val as c_int
    }

    #[inline]
    fn error_value() -> c_int {
        -1
    }
}


pub struct SuccessConverter;

impl CallbackConverter<bool> for SuccessConverter {
    type R = c_int;

    #[inline]
    fn convert(val: bool, _py: Python) -> c_int {
        if val { 0 } else { -1 }
    }

    #[inline]
    fn error_value() -> c_int {
        -1
    }
}


#[macro_export]
#[doc(hidden)]
macro_rules! py_class_call_slot {
    ($class:ident :: $f:ident [ $( { $pname:ident : $ptype:ty = $detail:tt } )* ]) => {{
        unsafe extern "C" fn wrap_call(
            slf: *mut $crate::_detail::ffi::PyObject,
            args: *mut $crate::_detail::ffi::PyObject,
            kwargs: *mut $crate::_detail::ffi::PyObject)
        -> *mut $crate::_detail::ffi::PyObject
        {
            const LOCATION: &'static str = concat!(stringify!($class), ".", stringify!($f), "()");
            $crate::_detail::handle_callback(
                LOCATION, $crate::_detail::PyObjectCallbackConverter,
                |py| {
                    py_argparse_raw!(py, Some(LOCATION), args, kwargs,
                        [ $( { $pname : $ptype = $detail } )* ]
                        {
                            let slf = $crate::PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<$class>();
                            let ret = slf.$f(py $(, $pname )* );
                            $crate::PyDrop::release_ref(slf, py);
                            ret
                        })
                })
        }
        Some(wrap_call)
    }}
}

/// Used as implementation in the `sq_item` slot to forward calls to the `mp_subscript` slot.
pub unsafe extern "C" fn sq_item(obj: *mut ffi::PyObject, index: ffi::Py_ssize_t) -> *mut ffi::PyObject {
    let arg = ffi::PyLong_FromSsize_t(index);
    if arg.is_null() {
        return arg;
    }
    let ret = ffi::PyObject_GetItem(obj, arg);
    ffi::Py_DECREF(arg);
    ret
}

