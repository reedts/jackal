use base64;
use ical::parser::ical::component::*;
use ical::parser::Component;
use ical::property::Property;
use serde::{ser, Serialize};
use std::collections::BTreeMap;
use std::default::Default;
use std::fmt::Display;

use crate::provider::{Error, ErrorKind, Result};

pub fn to_string(value: IcalCalendar) -> Result<String> {
    let mut serial = Serializer::default();
    serial.serialize_calendar(&value)?;
    serial.finish()
}

#[derive(Debug, Default)]
enum Position {
    Key,
    Parameters,
    Value,
    #[default]
    EOL,
}

#[derive(Debug)]
enum Section {
    Calendar,
    Timezones,
    TimezoneTransition(String),
    Alarms,
    Events,
}

impl Display for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Calendar => "VCALENDAR",
            Self::Alarms => "VALARM",
            Self::Events => "VEVENT",
            Self::Timezones => "VTIMEZONE",
            Self::TimezoneTransition(s) => s,
        };

        write!(f, "{}", s)
    }
}

pub struct Serializer {
    output: String,
    position: Position,
    section: Vec<Section>,
}

impl Serializer {
    fn begin_section(&mut self, sec: Section) -> Result<()> {
        match self.position {
            Position::EOL => {
                self.position = Position::Key;
                self.output += &format!("BEGIN:{}\n", &sec);
                self.section.push(sec);
                Ok(())
            }
            _ => Err(Error::new(
                ErrorKind::SerializeError,
                "Cannot begin new section: key value not finished",
            )),
        }
    }

    fn end_section(&mut self) -> Result<()> {
        match self.position {
            Position::EOL => {
                let sec = self.section.pop().unwrap();
                self.output += &format!("END:{}\n", sec);
                Ok(())
            }
            _ => {
                log::warn!("{}", &self.output);
                log::warn!("{:?}", self.section);
                Err(Error::new(
                    ErrorKind::SerializeError,
                    "Cannot end section: key value not finished",
                ))
            }
        }
    }

    fn serialize_properties(&mut self, value: &Vec<Property>) -> Result<()> {
        for Property {
            name,
            params,
            value,
        } in value
        {
            self.position = Position::Key;
            name.serialize(&mut *self)?;

            if let Some(p) = params {
                self.position = Position::Parameters;
                // This mimics the behaviour of "serialize_map",
                // however the structure itself is not a map.
                // Maybe we should serialize 2-tuple always this way?
                for (name, values) in p.iter() {
                    self.output += ";";
                    self.position = Position::Parameters;
                    name.serialize(&mut *self)?;
                    self.output += "=";
                    self.position = Position::Value;
                    values.serialize(&mut *self)?;
                }
            }

            self.output += ":";
            self.position = Position::Value;
            value.serialize(&mut *self)?;
            self.output += "\n";
            self.position = Position::EOL;
        }
        Ok(())
    }

    fn serialize_alarm(&mut self, value: &IcalAlarm) -> Result<()> {
        self.begin_section(Section::Alarms)?;
        self.serialize_properties(&value.properties)?;
        self.end_section()?;
        Ok(())
    }

    fn serialize_events(&mut self, value: &IcalEvent) -> Result<()> {
        self.begin_section(Section::Events)?;
        self.serialize_properties(&value.properties)?;

        // VEVENTS may encapsulate one or more VALARMs
        for alarm in value.alarms.iter() {
            self.serialize_alarm(&alarm)?;
        }

        self.end_section()?;
        Ok(())
    }

    fn serialize_timezones(&mut self, value: &IcalTimeZone) -> Result<()> {
        self.begin_section(Section::Timezones)?;
        self.serialize_properties(&value.properties)?;

        for transition in value.transitions.iter() {
            match transition.transition {
                Transition::Standard => {
                    self.begin_section(Section::TimezoneTransition("STANDARD".to_owned()))?
                }
                Transition::Daylight => {
                    self.begin_section(Section::TimezoneTransition("DAYLIGHT".to_owned()))?
                }
            }

            self.serialize_properties(&transition.properties)?;
            self.end_section()?;
        }

        self.end_section()?;
        Ok(())
    }

    fn serialize_calendar(&mut self, calendar: &IcalCalendar) -> Result<()> {
        // First serialize the properties of the calendar itself
        self.serialize_properties(&calendar.properties)?;

        for timezone in calendar.timezones.iter() {
            self.serialize_timezones(&timezone)?;
        }

        for event in calendar.events.iter() {
            self.serialize_events(&event)?;
        }

        Ok(())
    }

    pub fn finish(mut self) -> Result<String> {
        self.end_section()?;
        Ok(self.output)
    }
}

impl Default for Serializer {
    fn default() -> Self {
        let mut serial = Serializer {
            output: String::default(),
            position: Position::EOL,
            section: Vec::new(),
        };
        serial.begin_section(Section::Calendar).unwrap();
        serial
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.output += if v { "TRUE" } else { "FALSE" };
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.serialize_i32(v as i32)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.serialize_i32(v as i32)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.serialize_i32(v as i32)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        self.serialize_i32(v as i32)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        self.serialize_i32(v as i32)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        self.serialize_f32(v as f32)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.output += v;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        self.output += &base64::encode(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: ?Sized + Serialize,
    {
        variant.serialize(&mut *self)?;
        self.output += "=";
        value.serialize(&mut *self)?;
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        name.serialize(&mut *self)?;
        self.output += "=";
        variant.serialize(&mut *self)?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.serialize_tuple_variant(name, variant_index, variant, len)
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match &self.position {
            &Position::Value => {
                if !(self.output.ends_with(":") || self.output.ends_with("=")) {
                    self.output += ",";
                }
            }
            &Position::Parameters => {
                if !self.output.ends_with(";") {
                    self.output += ";";
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::SerializeError,
                    "Sequence not supported on this position",
                ))
            }
        }

        value.serialize(&mut **self)?;

        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeSeq>::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        <Self as ser::SerializeSeq>::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeSeq>::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        <Self as ser::SerializeSeq>::end(self)
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeTuple>::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        <Self as ser::SerializeTuple>::end(self)
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if !(self.output.ends_with(":") && self.output.ends_with(";")) {
            self.output += ";";
        }

        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += "=";
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeMap>::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeMap>::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}
