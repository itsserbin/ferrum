// SAFETY: These are raw Objective-C runtime C functions from libobjc.dylib.
// They have a stable ABI on macOS and are the foundation of the Objective-C runtime.
// Using raw FFI declarations avoids potential version conflicts with objc2 crate internals.
// All functions are well-documented in Apple's Objective-C Runtime Reference:
// - object_getClass: Returns the class of an object (always valid for valid objects)
// - sel_registerName: Registers a selector name and returns its unique identifier
// - class_addMethod: Adds a method to a class (returns false if method already exists)
// - class_replaceMethod: Replaces or adds a method implementation in a class
// - objc_msgSend: The universal message dispatch function for Objective-C
unsafe extern "C" {
    pub fn object_getClass(obj: *const core::ffi::c_void) -> *mut core::ffi::c_void;
    pub fn sel_registerName(name: *const core::ffi::c_char) -> *const core::ffi::c_void;
    pub fn class_addMethod(
        cls: *mut core::ffi::c_void,
        name: *const core::ffi::c_void,
        imp: unsafe extern "C" fn(),
        types: *const core::ffi::c_char,
    ) -> bool;
    pub fn class_replaceMethod(
        cls: *mut core::ffi::c_void,
        name: *const core::ffi::c_void,
        imp: unsafe extern "C" fn(),
        types: *const core::ffi::c_char,
    );
    pub fn objc_msgSend(
        obj: *const core::ffi::c_void,
        sel: *const core::ffi::c_void,
        ...
    ) -> *mut core::ffi::c_void;
}
