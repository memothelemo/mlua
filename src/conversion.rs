use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::convert::TryInto;
use std::ffi::{CStr, CString};
use std::hash::{BuildHasher, Hash};
use std::string::String as StdString;

use bstr::{BStr, BString};
use num_traits::cast;

use crate::error::{Error, Result};
use crate::function::Function;
use crate::lua::Lua;
use crate::string::String;
use crate::table::Table;
use crate::thread::Thread;
use crate::types::{LightUserData, MaybeSend};
use crate::userdata::{AnyUserData, UserData, UserDataRef, UserDataRefMut};
use crate::value::{FromLua, IntoLua, Nil, Value};

#[cfg(feature = "unstable")]
use crate::{
    function::{OwnedFunction, WrappedFunction},
    table::OwnedTable,
    userdata::OwnedAnyUserData,
};

#[cfg(all(feature = "async", feature = "unstable"))]
use crate::function::WrappedAsyncFunction;

impl IntoLua for Value {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(self)
    }
}

impl FromLua for Value {
    #[inline]
    fn from_lua(lua_value: Value, _: &Lua) -> Result<Self> {
        Ok(lua_value)
    }
}

impl IntoLua for String {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::String(self))
    }
}

impl FromLua for String {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<String> {
        let ty = value.type_name();
        lua.coerce_string(value)?
            .ok_or_else(|| Error::FromLuaConversionError {
                from: ty,
                to: "String",
                message: Some("expected string or number".to_string()),
            })
    }
}

impl IntoLua for Table {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Table(self))
    }
}

impl FromLua for Table {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Table> {
        match value {
            Value::Table(table) => Ok(table),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "table",
                message: None,
            }),
        }
    }
}

#[cfg(feature = "unstable")]
impl IntoLua for OwnedTable {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(Table(lua.adopt_owned_ref(self.0))))
    }
}

#[cfg(feature = "unstable")]
impl FromLua for OwnedTable {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<OwnedTable> {
        Table::from_lua(value, lua).map(|s| s.into_owned())
    }
}

impl IntoLua for Function {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Function(self))
    }
}

impl FromLua for Function {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Function> {
        match value {
            Value::Function(table) => Ok(table),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "function",
                message: None,
            }),
        }
    }
}

#[cfg(feature = "unstable")]
impl IntoLua for OwnedFunction {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Function(Function(lua.adopt_owned_ref(self.0))))
    }
}

#[cfg(feature = "unstable")]
impl FromLua for OwnedFunction {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<OwnedFunction> {
        Function::from_lua(value, lua).map(|s| s.into_owned())
    }
}

#[cfg(feature = "unstable")]
impl IntoLua for WrappedFunction {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        lua.create_callback(self.0).map(Value::Function)
    }
}

#[cfg(all(feature = "async", feature = "unstable"))]
impl IntoLua for WrappedAsyncFunction {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        lua.create_async_callback(self.0).map(Value::Function)
    }
}

impl IntoLua for Thread {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Thread(self))
    }
}

impl FromLua for Thread {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Thread> {
        match value {
            Value::Thread(t) => Ok(t),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "thread",
                message: None,
            }),
        }
    }
}

impl IntoLua for AnyUserData {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::UserData(self))
    }
}

impl FromLua for AnyUserData {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<AnyUserData> {
        match value {
            Value::UserData(ud) => Ok(ud),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "userdata",
                message: None,
            }),
        }
    }
}

#[cfg(feature = "unstable")]
impl IntoLua for OwnedAnyUserData {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::UserData(AnyUserData(lua.adopt_owned_ref(self.0))))
    }
}

#[cfg(feature = "unstable")]
impl FromLua for OwnedAnyUserData {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<OwnedAnyUserData> {
        AnyUserData::from_lua(value, lua).map(|s| s.into_owned())
    }
}

impl<'lua, T: 'static + MaybeSend + UserData> IntoLua for T {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::UserData(lua.create_userdata(self)?))
    }
}

impl<'lua, T: 'static> FromLua for UserDataRef<'lua, T> {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        Self::from_value(value)
    }
}

impl<'lua, T: 'static> FromLua for UserDataRefMut<'lua, T> {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        Self::from_value(value)
    }
}

impl IntoLua for Error {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Error(self))
    }
}

impl FromLua for Error {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Error> {
        match value {
            Value::Error(err) => Ok(err),
            val => Ok(Error::RuntimeError(
                lua.coerce_string(val)?
                    .and_then(|s| Some(s.to_str().ok()?.to_owned()))
                    .unwrap_or_else(|| "<unprintable error>".to_owned()),
            )),
        }
    }
}

impl IntoLua for bool {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Boolean(self))
    }
}

impl FromLua for bool {
    #[inline]
    fn from_lua(v: Value, _: &Lua) -> Result<Self> {
        match v {
            Value::Nil => Ok(false),
            Value::Boolean(b) => Ok(b),
            _ => Ok(true),
        }
    }
}

impl IntoLua for LightUserData {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::LightUserData(self))
    }
}

impl FromLua for LightUserData {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::LightUserData(ud) => Ok(ud),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "light userdata",
                message: None,
            }),
        }
    }
}

impl IntoLua for StdString {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(&self)?))
    }
}

impl FromLua for StdString {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let ty = value.type_name();
        Ok(lua
            .coerce_string(value)?
            .ok_or_else(|| Error::FromLuaConversionError {
                from: ty,
                to: "String",
                message: Some("expected string or number".to_string()),
            })?
            .to_str()?
            .to_owned())
    }
}

impl IntoLua for &str {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self)?))
    }
}

impl IntoLua for Cow<'_, str> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.as_bytes())?))
    }
}

impl IntoLua for Box<str> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(&*self)?))
    }
}

impl FromLua for Box<str> {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let ty = value.type_name();
        Ok(lua
            .coerce_string(value)?
            .ok_or_else(|| Error::FromLuaConversionError {
                from: ty,
                to: "Box<str>",
                message: Some("expected string or number".to_string()),
            })?
            .to_str()?
            .to_owned()
            .into_boxed_str())
    }
}

impl IntoLua for CString {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.as_bytes())?))
    }
}

impl FromLua for CString {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let ty = value.type_name();
        let string = lua
            .coerce_string(value)?
            .ok_or_else(|| Error::FromLuaConversionError {
                from: ty,
                to: "CString",
                message: Some("expected string or number".to_string()),
            })?;

        match CStr::from_bytes_with_nul(string.as_bytes_with_nul()) {
            Ok(s) => Ok(s.into()),
            Err(_) => Err(Error::FromLuaConversionError {
                from: ty,
                to: "CString",
                message: Some("invalid C-style string".to_string()),
            }),
        }
    }
}

impl IntoLua for &CStr {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.to_bytes())?))
    }
}

impl IntoLua for Cow<'_, CStr> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.to_bytes())?))
    }
}

impl IntoLua for BString {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(&self)?))
    }
}

impl FromLua for BString {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let ty = value.type_name();
        Ok(BString::from(
            lua.coerce_string(value)?
                .ok_or_else(|| Error::FromLuaConversionError {
                    from: ty,
                    to: "String",
                    message: Some("expected string or number".to_string()),
                })?
                .as_bytes()
                .to_vec(),
        ))
    }
}

impl IntoLua for &BStr {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self)?))
    }
}

macro_rules! lua_convert_int {
    ($x:ty) => {
        impl IntoLua for $x {
            #[inline]
            fn into_lua(self, _: &Lua) -> Result<Value> {
                cast(self)
                    .map(Value::Integer)
                    .or_else(|| cast(self).map(Value::Number))
                    // This is impossible error because conversion to Number never fails
                    .ok_or_else(|| Error::ToLuaConversionError {
                        from: stringify!($x),
                        to: "number",
                        message: Some("out of range".to_owned()),
                    })
            }
        }

        impl FromLua for $x {
            #[inline]
            fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
                let ty = value.type_name();
                (match value {
                    Value::Integer(i) => cast(i),
                    Value::Number(n) => cast(n),
                    _ => {
                        if let Some(i) = lua.coerce_integer(value.clone())? {
                            cast(i)
                        } else {
                            cast(lua.coerce_number(value)?.ok_or_else(|| {
                                Error::FromLuaConversionError {
                                    from: ty,
                                    to: stringify!($x),
                                    message: Some(
                                        "expected number or string coercible to number".to_string(),
                                    ),
                                }
                            })?)
                        }
                    }
                })
                .ok_or_else(|| Error::FromLuaConversionError {
                    from: ty,
                    to: stringify!($x),
                    message: Some("out of range".to_owned()),
                })
            }
        }
    };
}

lua_convert_int!(i8);
lua_convert_int!(u8);
lua_convert_int!(i16);
lua_convert_int!(u16);
lua_convert_int!(i32);
lua_convert_int!(u32);
lua_convert_int!(i64);
lua_convert_int!(u64);
lua_convert_int!(i128);
lua_convert_int!(u128);
lua_convert_int!(isize);
lua_convert_int!(usize);

macro_rules! lua_convert_float {
    ($x:ty) => {
        impl IntoLua for $x {
            #[inline]
            fn into_lua(self, _: &Lua) -> Result<Value> {
                cast(self)
                    .ok_or_else(|| Error::ToLuaConversionError {
                        from: stringify!($x),
                        to: "number",
                        message: Some("out of range".to_string()),
                    })
                    .map(Value::Number)
            }
        }

        impl FromLua for $x {
            #[inline]
            fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
                let ty = value.type_name();
                lua.coerce_number(value)?
                    .ok_or_else(|| Error::FromLuaConversionError {
                        from: ty,
                        to: stringify!($x),
                        message: Some("expected number or string coercible to number".to_string()),
                    })
                    .and_then(|n| {
                        cast(n).ok_or_else(|| Error::FromLuaConversionError {
                            from: ty,
                            to: stringify!($x),
                            message: Some("number out of range".to_string()),
                        })
                    })
            }
        }
    };
}

lua_convert_float!(f32);
lua_convert_float!(f64);

impl<'lua, T> IntoLua for &[T]
where
    T: Clone + IntoLua,
{
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(
            lua.create_sequence_from(self.iter().cloned())?,
        ))
    }
}

impl<'lua, T, const N: usize> IntoLua for [T; N]
where
    T: IntoLua,
{
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_sequence_from(self)?))
    }
}

impl<'lua, T, const N: usize> FromLua for [T; N]
where
    T: FromLua,
{
    #[inline]
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            #[cfg(feature = "luau")]
            Value::Vector(x, y, z) if N == 3 => Ok(mlua_expect!(
                vec![
                    T::from_lua(Value::Number(x as _), _lua)?,
                    T::from_lua(Value::Number(y as _), _lua)?,
                    T::from_lua(Value::Number(z as _), _lua)?,
                ]
                .try_into()
                .map_err(|_| ()),
                "cannot convert vector to array"
            )),
            Value::Table(table) => {
                let vec = table.sequence_values().collect::<Result<Vec<_>>>()?;
                vec.try_into()
                    .map_err(|vec: Vec<T>| Error::FromLuaConversionError {
                        from: "Table",
                        to: "Array",
                        message: Some(format!("expected table of length {}, got {}", N, vec.len())),
                    })
            }
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Array",
                message: Some("expected table".to_string()),
            }),
        }
    }
}

impl<'lua, T: IntoLua> IntoLua for Box<[T]> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_sequence_from(self.into_vec())?))
    }
}

impl<'lua, T: FromLua> FromLua for Box<[T]> {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        Ok(Vec::<T>::from_lua(value, lua)?.into_boxed_slice())
    }
}

impl<'lua, T: IntoLua> IntoLua for Vec<T> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_sequence_from(self)?))
    }
}

impl<'lua, T: FromLua> FromLua for Vec<T> {
    #[inline]
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            #[cfg(feature = "luau")]
            Value::Vector(x, y, z) => Ok(vec![
                T::from_lua(Value::Number(x as _), _lua)?,
                T::from_lua(Value::Number(y as _), _lua)?,
                T::from_lua(Value::Number(z as _), _lua)?,
            ]),
            Value::Table(table) => table.sequence_values().collect(),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Vec",
                message: Some("expected table".to_string()),
            }),
        }
    }
}

impl<'lua, K: Eq + Hash + IntoLua, V: IntoLua, S: BuildHasher> IntoLua for HashMap<K, V, S> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_table_from(self)?))
    }
}

impl<'lua, K: Eq + Hash + FromLua, V: FromLua, S: BuildHasher + Default> FromLua
    for HashMap<K, V, S>
{
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        if let Value::Table(table) = value {
            table.pairs().collect()
        } else {
            Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "HashMap",
                message: Some("expected table".to_string()),
            })
        }
    }
}

impl<'lua, K: Ord + IntoLua, V: IntoLua> IntoLua for BTreeMap<K, V> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_table_from(self)?))
    }
}

impl<'lua, K: Ord + FromLua, V: FromLua> FromLua for BTreeMap<K, V> {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        if let Value::Table(table) = value {
            table.pairs().collect()
        } else {
            Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "BTreeMap",
                message: Some("expected table".to_string()),
            })
        }
    }
}

impl<'lua, T: Eq + Hash + IntoLua, S: BuildHasher> IntoLua for HashSet<T, S> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_table_from(
            self.into_iter().map(|val| (val, true)),
        )?))
    }
}

impl<'lua, T: Eq + Hash + FromLua, S: BuildHasher + Default> FromLua for HashSet<T, S> {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::Table(table) if table.len()? > 0 => table.sequence_values().collect(),
            Value::Table(table) => table
                .pairs::<T, Value>()
                .map(|res| res.map(|(k, _)| k))
                .collect(),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "HashSet",
                message: Some("expected table".to_string()),
            }),
        }
    }
}

impl<'lua, T: Ord + IntoLua> IntoLua for BTreeSet<T> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_table_from(
            self.into_iter().map(|val| (val, true)),
        )?))
    }
}

impl<'lua, T: Ord + FromLua> FromLua for BTreeSet<T> {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::Table(table) if table.len()? > 0 => table.sequence_values().collect(),
            Value::Table(table) => table
                .pairs::<T, Value>()
                .map(|res| res.map(|(k, _)| k))
                .collect(),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "BTreeSet",
                message: Some("expected table".to_string()),
            }),
        }
    }
}

impl<'lua, T: IntoLua> IntoLua for Option<T> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        match self {
            Some(val) => val.into_lua(lua),
            None => Ok(Nil),
        }
    }
}

impl<'lua, T: FromLua> FromLua for Option<T> {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        match value {
            Nil => Ok(None),
            value => Ok(Some(T::from_lua(value, lua)?)),
        }
    }
}
