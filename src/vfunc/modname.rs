/*
typedef void (*duckdb_scalar_function_t)(duckdb_function_info info, duckdb_data_chunk input, duckdb_vector output);

duckdb_scalar_function duckdb_create_scalar_function();
void duckdb_destroy_scalar_function(duckdb_scalar_function *scalar_function);
void duckdb_scalar_function_set_name(duckdb_scalar_function scalar_function, const char *name);
void duckdb_scalar_function_add_parameter(duckdb_scalar_function scalar_function, duckdb_logical_type type);
void duckdb_scalar_function_set_return_type(duckdb_scalar_function scalar_function, duckdb_logical_type type);
void duckdb_scalar_function_set_extra_info(duckdb_scalar_function scalar_function, void *extra_info,
                                                     duckdb_delete_callback_t destroy);
void duckdb_scalar_function_set_function(duckdb_scalar_function scalar_function,
                                                    duckdb_scalar_function_t function);
duckdb_state duckdb_register_scalar_function(duckdb_connection con, duckdb_scalar_function scalar_function);
 */

use libduckdb_sys::{
    duckdb_connection, duckdb_data_chunk, duckdb_delete_callback_t, duckdb_function_info, duckdb_logical_type,
    duckdb_vector,
};
use std::ffi::{c_char, c_void};

#[allow(non_camel_case_types)]
pub(crate) type duckdb_scalar_function = *mut c_void;

#[allow(non_camel_case_types)]
pub(crate) type duckdb_scalar_function_t =
    unsafe extern "C" fn(*mut duckdb_function_info, *mut duckdb_data_chunk, *mut duckdb_vector);

extern "C" {
    pub(crate) fn duckdb_create_scalar_function() -> duckdb_scalar_function;
}

extern "C" {
    pub(crate) fn duckdb_scalar_function_set_name(func: duckdb_scalar_function, name: *const c_char);
}

extern "C" {
    pub(crate) fn duckdb_scalar_function_set_function(func: duckdb_scalar_function, function: duckdb_scalar_function_t);
}

extern "C" {
    pub(crate) fn duckdb_scalar_function_add_parameter(func: duckdb_scalar_function, ptr: duckdb_logical_type);
}

extern "C" {
    pub(crate) fn duckdb_scalar_function_set_return_type(func: duckdb_scalar_function, ptr: duckdb_logical_type);
}

extern "C" {
    pub(crate) fn duckdb_scalar_function_set_extra_info(
        func: duckdb_scalar_function,
        extra_info: *mut c_void,
        destroy: duckdb_delete_callback_t,
    );
}

extern "C" {
    #[must_use]
    pub(crate) fn duckdb_register_scalar_function(
        con: duckdb_connection,
        scalar_function: duckdb_scalar_function,
    ) -> libduckdb_sys::duckdb_state;
}
