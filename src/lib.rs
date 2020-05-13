use librrb::{Iter as VIter, IterMut as VIterMut, Vector};
use serde_json::{Number as JsonNumber, Value as JsonValue};
use std::borrow::Borrow;
use std::fmt::{self, Debug, Display};
use std::iter::FusedIterator;
use std::mem;
use std::ops;

mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl<'a, T> Sealed for &'a T where T: ?Sized + Sealed {}
}

pub trait Index: private::Sealed {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value>;

    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value>;

    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value;
}

impl Index for usize {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Array(arr) => arr.get(*self),
            _ => None,
        }
    }

    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Array(arr) => arr.get_mut(*self),
            _ => None,
        }
    }

    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        match v {
            Value::Array(arr) => arr.get_mut(*self).unwrap(),
            _ => panic!(),
        }
    }
}

impl Index for str {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Object(obj) => obj.get(self),
            _ => None,
        }
    }

    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Object(obj) => obj.get_mut(self),
            _ => None,
        }
    }

    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        match v {
            Value::Object(obj) => obj.entry(self).or_insert(Value::Null),
            _ => panic!(),
        }
    }
}

impl Index for String {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        self[..].index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        self[..].index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        self[..].index_or_insert(v)
    }
}

impl<'a, T> Index for &'a T
where
    T: ?Sized + Index,
{
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        (**self).index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        (**self).index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        (**self).index_or_insert(v)
    }
}

impl<I> ops::Index<I> for Value
where
    I: Index,
{
    type Output = Value;
    fn index(&self, index: I) -> &Value {
        index.index_into(self).unwrap_or(&Value::Null)
    }
}

impl<I> ops::IndexMut<I> for Value
where
    I: Index,
{
    fn index_mut(&mut self, index: I) -> &mut Value {
        index.index_or_insert(self)
    }
}

pub struct VacantEntry<'a>
where
    String: 'a,
{
    keys: &'a mut Vector<String>,
    values: &'a mut Vector<Value>,
    key: String,
    idx: usize,
}

impl<'a> VacantEntry<'a> {
    pub fn key(&self) -> &String {
        &self.key
    }

    pub fn insert(self, value: Value) -> &'a mut Value {
        self.keys.insert(self.idx, self.key);
        self.values.insert(self.idx, value);
        self.values.get_mut(self.idx).unwrap()
    }
}

pub struct OccupiedEntry<'a> {
    keys: &'a mut Vector<String>,
    values: &'a mut Vector<Value>,
    idx: usize,
}

impl<'a> OccupiedEntry<'a> {
    pub fn key(&self) -> &String {
        self.keys.get(self.idx).unwrap()
    }

    pub fn get(&self) -> &Value {
        self.values.get(self.idx).unwrap()
    }

    pub fn get_mut(&mut self) -> &mut Value {
        self.values.get_mut(self.idx).unwrap()
    }
    pub fn into_mut(self) -> &'a mut Value {
        self.values.get_mut(self.idx).unwrap()
    }

    pub fn insert(&mut self, value: Value) -> Value {
        mem::replace(self.get_mut(), value)
    }

    pub fn remove(&mut self) -> Value {
        self.keys.remove(self.idx).unwrap();
        self.values.remove(self.idx).unwrap()
    }
}

pub enum Entry<'a> {
    Vacant(VacantEntry<'a>),
    Occupied(OccupiedEntry<'a>),
}

impl<'a> Entry<'a> {
    pub fn key(&self) -> &String {
        match self {
            Entry::Vacant(e) => e.key(),
            Entry::Occupied(e) => e.key(),
        }
    }

    pub fn or_insert(self, default: Value) -> &'a mut Value {
        match self {
            Entry::Vacant(e) => e.insert(default),
            Entry::Occupied(e) => e.into_mut(),
        }
    }

    pub fn or_insert_with<F>(self, default: F) -> &'a mut Value
    where
        F: FnOnce() -> Value,
    {
        match self {
            Entry::Vacant(e) => e.insert(default()),
            Entry::Occupied(e) => e.into_mut(),
        }
    }
}

pub struct Iter<'a> {
    key: VIter<'a, String>,
    value: VIter<'a, Value>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a String, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(key) = self.key.next() {
            Some((key, self.value.next().unwrap()))
        } else {
            debug_assert!(self.value.next().is_none());
            None
        }
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(key) = self.key.next_back() {
            Some((key, self.value.next_back().unwrap()))
        } else {
            debug_assert!(self.value.next_back().is_none());
            None
        }
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {}
impl<'a> FusedIterator for Iter<'a> {}

pub struct IterMut<'a> {
    key: VIter<'a, String>,
    value: VIterMut<'a, Value>,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = (&'a String, &'a mut Value);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(key) = self.key.next() {
            Some((key, self.value.next().unwrap()))
        } else {
            debug_assert!(self.value.next().is_none());
            None
        }
    }
}

impl<'a> DoubleEndedIterator for IterMut<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(key) = self.key.next_back() {
            Some((key, self.value.next_back().unwrap()))
        } else {
            debug_assert!(self.value.next_back().is_none());
            None
        }
    }
}

impl<'a> ExactSizeIterator for IterMut<'a> {}
impl<'a> FusedIterator for IterMut<'a> {}

pub type Keys<'a> = VIter<'a, String>;
pub type Values<'a> = VIter<'a, Value>;
pub type ValuesMut<'a> = VIterMut<'a, Value>;

#[derive(Clone, Debug, Default, PartialOrd, PartialEq)]
pub struct Object {
    keys: Vector<String>,
    values: Vector<Value>,
}

impl Object {
    pub fn new() -> Self {
        Object {
            keys: Vector::new(),
            values: Vector::new(),
        }
    }

    pub fn clear(&mut self) {
        unimplemented!()
    }

    fn get_index_for_key<Q>(&self, key: &Q) -> Result<usize, usize>
    where
        String: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self.keys.equal_range(key) {
            Ok(range) => {
                debug_assert_eq!(range.len(), 1);
                Ok(range.start)
            }
            Err(position) => Err(position),
        }
    }

    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&Value>
    where
        String: Borrow<Q>,
        Q: Ord,
    {
        self.get_index_for_key(key)
            .ok()
            .and_then(move |v| self.values.get(v))
    }

    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut Value>
    where
        String: Borrow<Q>,
        Q: Ord,
    {
        self.get_index_for_key(key)
            .ok()
            .and_then(move |v| self.values.get_mut(v))
    }

    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        String: Borrow<Q>,
        Q: Ord + Eq,
    {
        self.get_index_for_key(key).is_ok()
    }

    pub fn insert(&mut self, k: String, v: Value) -> Option<Value> {
        let position = self.get_index_for_key(&k);
        match position {
            Ok(position) => {
                let existing_value_ref = self.values.get_mut(position).unwrap();
                Some(mem::replace(existing_value_ref, v))
            }
            Err(position) => {
                self.keys.insert(position, k);
                self.values.insert(position, v);
                None
            }
        }
    }

    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<Value>
    where
        String: Borrow<Q>,
        Q: Ord + Eq,
    {
        let position = self.get_index_for_key(key);
        match position {
            Ok(position) => {
                self.keys.remove(position);
                self.values.remove(position)
            }
            Err(_) => None,
        }
    }

    pub fn append(&mut self, other: &mut Self) {
        let other_keys = mem::replace(&mut other.keys, Vector::new());
        let other_values = mem::replace(&mut other.values, Vector::new());
        self.keys.append(other_keys);
        self.values.append(other_values);
        self.keys.dual_sort(&mut self.values)
    }

    pub fn entry<S>(&mut self, key: S) -> Entry
    where
        S: Into<String>,
    {
        let string = key.into();
        let idx = self.get_index_for_key(&string);
        match idx {
            Ok(idx) => Entry::Occupied(OccupiedEntry {
                idx,
                keys: &mut self.keys,
                values: &mut self.values,
            }),
            Err(idx) => Entry::Vacant(VacantEntry {
                idx,
                key: string,
                keys: &mut self.keys,
                values: &mut self.values,
            }),
        }
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn iter(&self) -> Iter {
        Iter {
            key: self.keys.iter(),
            value: self.values.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            key: self.keys.iter(),
            value: self.values.iter_mut(),
        }
    }

    pub fn keys(&self) -> Keys {
        self.keys.iter()
    }

    pub fn values(&self) -> Values {
        self.values.iter()
    }

    pub fn values_mut(&mut self) -> ValuesMut {
        self.values.iter_mut()
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub enum Number {
    PosInt(u64),
    /// Always less than zero.
    NegInt(i64),
    /// Always finite.
    Float(f64),
}

impl Number {
    pub fn is_i64(&self) -> bool {
        match self {
            Number::PosInt(v) => *v <= i64::max_value() as u64,
            Number::NegInt(_) => true,
            Number::Float(_) => false,
        }
    }

    pub fn is_u64(&self) -> bool {
        match self {
            Number::PosInt(_) => true,
            Number::NegInt(_) | Number::Float(_) => false,
        }
    }

    pub fn is_f64(&self) -> bool {
        match self {
            Number::Float(_) => true,
            Number::PosInt(_) | Number::NegInt(_) => false,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Number::PosInt(n) => {
                if *n <= i64::max_value() as u64 {
                    Some(*n as i64)
                } else {
                    None
                }
            }
            Number::NegInt(n) => Some(*n),
            Number::Float(_) => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Number::PosInt(n) => Some(*n),
            Number::NegInt(_) | Number::Float(_) => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Number::PosInt(n) => Some(*n as f64),
            Number::NegInt(n) => Some(*n as f64),
            Number::Float(n) => Some(*n),
        }
    }

    pub fn from_f64(f: f64) -> Option<Number> {
        if f.is_finite() {
            Some(Number::Float(f))
        } else {
            None
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Number::PosInt(u) => Display::fmt(&u, formatter),
            Number::NegInt(i) => Display::fmt(&i, formatter),
            Number::Float(f) => Display::fmt(&f, formatter),
        }
    }
}

impl Debug for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let mut debug = formatter.debug_tuple("Number");
        match self {
            Number::PosInt(i) => {
                debug.field(&i);
            }
            Number::NegInt(i) => {
                debug.field(&i);
            }
            Number::Float(f) => {
                debug.field(&f);
            }
        }
        debug.finish()
    }
}

impl From<JsonNumber> for Number {
    fn from(n: JsonNumber) -> Number {
        if n.is_f64() {
            Number::Float(n.as_f64().unwrap())
        } else if n.is_u64() {
            Number::PosInt(n.as_u64().unwrap())
        } else {
            Number::NegInt(n.as_i64().unwrap())
        }
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum Value {
    Null,
    Number(Number),
    String(String),
    Bool(bool),
    Array(Vector<Value>),
    Object(Object),
}

impl Value {
    pub fn is_null(&self) -> bool {
        match self {
            Value::Null => true,
            _ => false,
        }
    }

    pub fn as_null(&self) -> Option<()> {
        match self {
            Value::Null => Some(()),
            _ => None,
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            Value::Number(_) => true,
            _ => false,
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            Value::String(_) => true,
            _ => false,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(&s),
            _ => None,
        }
    }

    pub fn is_boolean(&self) -> bool {
        match self {
            Value::Bool(_) => true,
            _ => false,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            Value::Array(_) => true,
            _ => false,
        }
    }

    pub fn as_array(&self) -> Option<&Vector<Value>> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vector<Value>> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            Value::Object(_) => true,
            _ => false,
        }
    }

    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Object> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl From<JsonValue> for Value {
    fn from(v: JsonValue) -> Value {
        match v {
            JsonValue::Null => Value::Null,
            JsonValue::Number(n) => Value::Number(n.into()),
            JsonValue::String(s) => Value::String(s),
            JsonValue::Bool(b) => Value::Bool(b),
            JsonValue::Array(arr) => {
                let mut v = Vector::new();
                for item in arr {
                    v.push_back(item.into())
                }
                Value::Array(v)
            }
            JsonValue::Object(obj) => {
                let mut o = Object::new();
                for (k, v) in obj {
                    o.insert(k, v.into());
                }
                Value::Object(o)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn it_works() {
        let values: Value = json!(null).into();
        assert!(values.eq(&Value::Null));
        let values: Value = json!(true).into();
        assert!(values.eq(&Value::Bool(true)));
        let values: Value = json!(false).into();
        assert!(values.eq(&Value::Bool(false)));
        let values: Value = json!(5).into();
        assert!(values.eq(&Value::Number(Number::PosInt(5))));
        let values: Value = json!(5.0).into();
        assert!(values.eq(&Value::Number(Number::Float(5.0))));
        let values: Value = json!(-5).into();
        assert!(values.eq(&Value::Number(Number::NegInt(-5))));
        let values: Value = json!("lol").into();
        assert!(values.eq(&Value::String("lol".to_owned())));
        let values: Value = json!([true, true, true, true, true]).into();
        assert!(values.eq(&Value::Array(Vector::constant_vec_of_length(
            Value::Bool(true),
            5
        ))));
        let values: Value = json!({}).into();
        assert!(values.eq(&Value::Object(Object::new())));
    }
}
