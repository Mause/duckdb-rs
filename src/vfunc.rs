use crate::{
    inner_connection::InnerConnection,
    vtab::{drop_data_c, DataChunk, FlatVector, Free, FunctionInfo, LogicalType},
    Connection, Error,
};
use libduckdb_sys as ffi;
use libduckdb_sys::{duckdb_data_chunk, duckdb_function_info, duckdb_vector};
use std::ffi::CString;

mod modname;

use self::modname::{
    duckdb_create_scalar_function, duckdb_register_scalar_function, duckdb_scalar_function,
    duckdb_scalar_function_add_parameter, duckdb_scalar_function_set_extra_info, duckdb_scalar_function_set_function,
    duckdb_scalar_function_set_name, duckdb_scalar_function_set_return_type, duckdb_scalar_function_t,
};

/// The duckdb scalar function interface
pub trait VFunc: Sized {
    /// The actual function
    ///
    /// # Safety
    ///
    /// This function is unsafe because it:
    ///
    /// - Dereferences multiple raw pointers (`func` to access `init_info` and `bind_info`).
    ///
    /// The caller must ensure that:
    ///
    /// - All pointers (`func`, `output`, internal `init_info`, and `bind_info`) are valid and point to the expected types of data structures.
    /// - The `init_info` and `bind_info` data pointed to remains valid and is not freed until after this function completes.
    /// - No other threads are concurrently mutating the data pointed to by `init_info` and `bind_info` without proper synchronization.
    /// - The `output` parameter is correctly initialized and can safely be written to.
    unsafe fn func(
        func: &FunctionInfo,
        input: &mut DataChunk,
        output: &mut FlatVector,
    ) -> crate::Result<(), Box<dyn std::error::Error>>;

    /// The function return type
    fn return_type() -> LogicalType;

    /// The function parameters
    fn parameters() -> Option<Vec<LogicalType>> {
        None
    }
}

unsafe extern "C" fn virtual_function<Func>(
    function_info: *mut duckdb_function_info,
    input: *mut duckdb_data_chunk,
    output: *mut duckdb_vector,
) where
    Func: VFunc,
{
    let function_info = FunctionInfo::from(*function_info);
    let mut input = DataChunk::from(*input);
    let mut output = FlatVector::from(*output);
    if let Err(err) = Func::func(&function_info, &mut input, &mut output) {
        function_info.set_error(err.to_string().as_ref());
    }
}

impl Connection {
    /// Register a scalar function
    pub fn register_scalar_function<Func: VFunc>(&self, name: &str) -> crate::Result<()> {
        let mut func = ScalarFunction::new();
        func.set_name(name)
            .set_function(virtual_function::<Func>)
            .set_return_type(Func::return_type());
        for param in Func::parameters().unwrap_or_default() {
            func.add_parameter(param);
        }
        self.db.borrow_mut().register_scalar_function(func)
    }
}

impl InnerConnection {
    /// Register the given ScalarFunction with the current db
    pub fn register_scalar_function(&mut self, scalar_function: ScalarFunction) -> crate::Result<()> {
        unsafe {
            let rc = duckdb_register_scalar_function(self.con, scalar_function.0);
            if rc != ffi::DuckDBSuccess {
                return Err(Error::DuckDBFailure(ffi::Error::new(rc), None));
            }
        }
        Ok(())
    }
}

/// A scalar function that can be added to a database connection to register a function
pub struct ScalarFunction(duckdb_scalar_function);

impl ScalarFunction {
    fn new() -> Self {
        ScalarFunction(unsafe { duckdb_create_scalar_function() })
    }
    /// Set the name of the scalar function
    pub fn set_name(&mut self, name: &str) -> &mut Self {
        unsafe {
            let name = &CString::new(name).unwrap();
            duckdb_scalar_function_set_name(self.0, name.as_ptr());
            self
        }
    }
    /// Add a parameter to the scalar function
    pub fn add_parameter(&mut self, param: LogicalType) -> &mut Self {
        unsafe {
            duckdb_scalar_function_add_parameter(self.0, param.ptr);
            self
        }
    }
    /// Set the return type of the scalar function
    pub fn set_function(&mut self, function: duckdb_scalar_function_t) -> &mut Self {
        unsafe {
            duckdb_scalar_function_set_function(self.0, function);
            self
        }
    }
    /// Set the return type of the scalar function
    pub fn set_return_type(&mut self, return_type: LogicalType) -> &mut Self {
        unsafe {
            duckdb_scalar_function_set_return_type(self.0, return_type.ptr);
            self
        }
    }
    /// Set the extra info of the scalar function
    pub fn set_extra_info<T>(&mut self, extra_info: *mut T) -> &mut Self
    where
        T: Sized + Free,
    {
        unsafe {
            duckdb_scalar_function_set_extra_info(self.0, extra_info.cast(), Some(drop_data_c::<T>));
            self
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        vtab::{malloc_data_c, DataChunk, FlatVector, Free, FunctionInfo, LogicalType, LogicalTypeId},
        Connection,
    };

    use super::{ScalarFunction, VFunc};

    struct BasicFunc;

    impl VFunc for BasicFunc {
        fn return_type() -> LogicalType {
            LogicalType::new(LogicalTypeId::Integer)
        }
        fn parameters() -> Option<Vec<LogicalType>> {
            Some(vec![LogicalType::new(LogicalTypeId::Integer)])
        }
        unsafe fn func(
            _: &FunctionInfo,
            input: &mut DataChunk,
            output: &mut FlatVector,
        ) -> crate::Result<(), Box<dyn std::error::Error>> {
            let mut input = input.flat_vector(0);
            let output = output.as_mut_slice::<i64>();
            let input = input.as_mut_slice::<i64>();
            for i in 0..input.len() {
                output[i] = input[i] * 2;
            }
            Ok(())
        }
    }

    #[test]
    fn test_basic_function() -> Result<(), Box<dyn std::error::Error>> {
        let db = Connection::open_in_memory()?;
        db.register_scalar_function::<BasicFunc>("basic_func")?;

        let row: i64 = db.query_row("SELECT basic_func(1)", [], |row| row.get(0))?;
        assert_eq!(row, 2);

        Ok(())
    }

    #[repr(C)]
    struct ExtraInfoStruct(i64);

    impl Free for ExtraInfoStruct {}

    struct ExtraInfoFunc;

    impl VFunc for ExtraInfoFunc {
        unsafe fn func(
            func: &FunctionInfo,
            input: &mut DataChunk,
            output: &mut FlatVector,
        ) -> crate::Result<(), Box<dyn std::error::Error>> {
            let mut input = input.flat_vector(0);
            let output = output.as_mut_slice::<i64>();
            let input = input.as_mut_slice::<i64>();
            for i in 0..input.len() {
                output[i] = input[i] * (*func.get_extra_info::<ExtraInfoStruct>()).0;
            }
            Ok(())
        }

        fn parameters() -> Option<Vec<LogicalType>> {
            Some(vec![LogicalType::new(LogicalTypeId::Integer)])
        }

        fn return_type() -> LogicalType {
            LogicalType::new(LogicalTypeId::Integer)
        }
    }

    #[test]
    fn test_extra_info() -> Result<(), Box<dyn std::error::Error>> {
        let mut func = ScalarFunction::new();
        let extra_info: *mut ExtraInfoStruct;
        unsafe {
            extra_info = malloc_data_c::<ExtraInfoStruct>();
            (*extra_info).0 = 10;
        }
        func.set_name("name")
            .set_return_type(LogicalType::new(LogicalTypeId::Integer))
            .set_extra_info(extra_info);
        let db = Connection::open_in_memory()?;
        db.db.borrow_mut().register_scalar_function(func)?;

        let row: i64 = db.query_row("SELECT name(1)", [], |r| r.get(0))?;

        assert_eq!(row, 100);

        Ok(())
    }
}
