mod deserialize;
#[allow(unused)]
mod json_capnp;
pub mod serialize;
pub mod typescript;

pub use deserialize::deserialize_into;
pub use serialize::serialize;

use capnp::{
    schema::{AnnotationList, Enumerant, Field, StructSchema},
    Result,
};

use self::json_capnp::{discriminator_options, flatten_options};

// TODO: Support flatten
// TODO: support discriminator annotations

#[derive(Debug, Default)]
struct JsonAnnots<'a> {
    name: Option<&'a str>,
    data_format: Option<DataFormat>,
    flatten: Option<FlattenOptions<'a>>,
}

#[derive(Debug, Clone, Copy)]
enum DataFormat {
    Hex,
    Base64,
}

#[derive(Debug, Default)]
struct FlattenOptions<'a> {
    prefix: Option<&'a str>,
}

fn read_annots<'a>(list: AnnotationList) -> Result<JsonAnnots<'a>> {
    let mut out = JsonAnnots::default();
    for annot in list.iter() {
        match annot.get_id() {
            json_capnp::name::ID => {
                let value: &str = annot.get_value()?.downcast();
                out.name = Some(value);
            }
            json_capnp::flatten::ID => {
                let reader: flatten_options::Reader<'_> = annot.get_value()?.downcast();
                let mut value = FlattenOptions::default();
                if reader.has_prefix() {
                    value.prefix = Some(reader.get_prefix()?);
                }
            }
            json_capnp::base64::ID => out.data_format = Some(DataFormat::Base64),
            json_capnp::hex::ID => out.data_format = Some(DataFormat::Hex),
            _ => {}
        }
    }
    Ok(out)
}

#[derive(Debug, Default)]
struct DiscriminatorOptions {
    name: &'static str,
    value_name: Option<&'static str>,
}

fn read_discriminator(schema: StructSchema) -> Result<DiscriminatorOptions> {
    let mut value = DiscriminatorOptions::default();

    let annots = schema.get_annotations()?;
    let Some(annot) = annots.find(json_capnp::discriminator::ID) else {
        return Ok(value)
    };
    let reader: discriminator_options::Reader<'_> = annot.get_value()?.downcast();

    if reader.has_name() {
        value.name = reader.get_name()?;
    } else {
        if let Some(union_name) = annots.find(json_capnp::name::ID) {
            value.name = union_name.get_value()?.downcast();
        } else {
            value.name = schema.name()?;
        }
    }

    if reader.has_value_name() {
        value.value_name = Some(reader.get_value_name()?);
    }

    Ok(value)
}

fn field_name<'a>(field: &'a Field, annots: &'a JsonAnnots) -> Result<&'a str> {
    if let Some(name) = annots.name {
        Ok(name)
    } else {
        field.name()
    }
}

fn enumerant_value(enumerant: &Enumerant) -> Result<&str> {
    let proto = enumerant.get_proto();
    let annots = read_annots(enumerant.get_annotations()?)?;
    if let Some(name) = annots.name {
        Ok(name)
    } else {
        proto.get_name()
    }
}
