//! Generate a typescript interface for the JSON that will be serialized and
//! deserialized.
//!
//! ```
//! # fn main() -> capnp::Result<()> {
//! # use capnp::schema_capnp::{value as your_schema, field as your_other_schema};
//! let mut ctx = capnp_json::typescript::Context::new(Default::default());
//! ctx.add::<your_schema::Owned>()?;
//! ctx.add::<your_other_schema::Owned>()?;
//! let typescript_code = ctx.write();
//! # Ok(())
//! # }
//! ```

use std::{
    collections::{BTreeMap, HashSet},
    fmt,
};

use capnp::{
    introspect::{Introspect, Type, TypeVariant},
    schema::{EnumSchema, Field, StructSchema},
    traits::OwnedStruct,
    Error, Result,
};

use crate::{
    enumerant_value, field_name, json_capnp, read_annots, read_discriminator,
    serialize::{self, OnEnumerantNotInSchema},
    DataFormat, JsonAnnots,
};

#[derive(Debug, Default)]
pub struct Context {
    opts: serialize::Opts,
    items: Vec<Item>,
    visited: HashSet<TypeIdent>,
}

#[derive(Debug)]
enum Item {
    Alias(TypeIdent, TypeIdent),
    Interface {
        ident: TypeIdent,
        fields: Vec<(String, TypeIdent)>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
enum TypeIdent {
    Null,
    Bool,
    Number,
    String,
    Literal(String),
    Base64Data,
    HexData,
    Enum(String),
    Struct(String),
    Union(Vec<TypeIdent>),
    ArrayOf(Box<TypeIdent>),
}

impl Context {
    pub fn new(opts: serialize::Opts) -> Self {
        Self {
            opts,
            visited: HashSet::new(),
            items: Vec::new(),
        }
    }

    /// Retursively add this struct and anything it references to the generated
    /// code.
    ///
    /// ```
    /// # fn main() -> capnp::Result<()> {
    /// # use capnp::schema_capnp::{value as your_schema};
    /// let mut ctx = capnp_json::typescript::Context::new(Default::default());
    /// ctx.add::<your_schema::Owned>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add<Owned>(&mut self) -> Result<()>
    where
        Owned: OwnedStruct + Introspect,
    {
        let TypeVariant::Struct(schema) = Owned::introspect().which() else {
            unreachable!()
        };
        self.visit_struct(schema.into())?;
        Ok(())
    }

    /// Write out the generated code.
    pub fn write(&self) -> String {
        let mut lines = vec!["".into()];
        for item in &self.items {
            item.write_to(&mut lines);
        }
        lines.join("\n")
    }

    fn visit_type(
        &mut self,
        schema: Type,
        field: Option<&JsonAnnots>,
    ) -> Result<Option<TypeIdent>> {
        match schema.which() {
            capnp::introspect::TypeVariant::Void => Ok(Some(TypeIdent::Null)),
            capnp::introspect::TypeVariant::Bool => Ok(Some(TypeIdent::Bool)),
            capnp::introspect::TypeVariant::Int8
            | capnp::introspect::TypeVariant::Int16
            | capnp::introspect::TypeVariant::Int32
            | capnp::introspect::TypeVariant::Int64
            | capnp::introspect::TypeVariant::UInt8
            | capnp::introspect::TypeVariant::UInt16
            | capnp::introspect::TypeVariant::UInt32
            | capnp::introspect::TypeVariant::UInt64
            | capnp::introspect::TypeVariant::Float32
            | capnp::introspect::TypeVariant::Float64 => Ok(Some(TypeIdent::Number)),
            capnp::introspect::TypeVariant::Text => Ok(Some(TypeIdent::String)),
            capnp::introspect::TypeVariant::Data => {
                let field = field.expect("visit_type for Data must be on field");
                self.visit_data(field)
            }
            capnp::introspect::TypeVariant::Struct(schema) => {
                self.visit_struct(schema.into()).map(Some)
            }
            capnp::introspect::TypeVariant::AnyPointer => Ok(None),
            capnp::introspect::TypeVariant::Capability => Ok(None),
            capnp::introspect::TypeVariant::Enum(schema) => {
                self.visit_enum(schema.into()).map(Some)
            }
            capnp::introspect::TypeVariant::List(schema) => {
                if let Some(elem_type) = self.visit_type(schema, field)? {
                    Ok(Some(TypeIdent::ArrayOf(Box::new(elem_type))))
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn visit_data(&mut self, field: &JsonAnnots) -> Result<Option<TypeIdent>> {
        let Some(format) = field.data_format else { return Ok(None) };
        let ident = TypeIdent::for_data(format);
        if !self.visited.contains(&ident) {
            self.visited.insert(ident.clone());
            self.items
                .push(Item::Alias(ident.clone(), TypeIdent::String));
        }
        Ok(Some(ident))
    }

    fn visit_struct(&mut self, schema: StructSchema) -> Result<TypeIdent> {
        let ident = TypeIdent::from_struct(schema)?;

        if self.visited.contains(&ident) {
            return Ok(ident);
        }
        self.visited.insert(ident.clone());

        let mut non_union_fields = BTreeMap::new();
        for field in schema.get_non_union_fields()? {
            let annots = read_annots(field.get_annotations()?)?;
            let name = field_name(&field, &annots)?.to_string();
            if let Some(type_ident) = self.visit_type(field.get_type(), Some(&annots))? {
                non_union_fields.insert(field.code_order(), (name, type_ident));
            }
        }
        let non_union_fields = non_union_fields.into_values().collect::<Vec<_>>();

        if schema.has_union_fields() {
            let opts = read_discriminator(schema)?;
            let mut variants = Vec::new();

            for field in schema.get_union_fields()? {
                let annots = read_annots(field.get_annotations()?)?;
                let Some(type_id) = self.visit_type(field.get_type(), Some(&annots))? else { continue };
                let variant_ident = TypeIdent::from_union_variant(schema, field)?;

                let name = if let Some(value_name) = opts.value_name {
                    value_name
                } else {
                    field_name(&field, &annots)?
                };

                let mut variant_fields = non_union_fields.clone();
                variant_fields.push((name.to_string(), type_id));
                self.items.push(Item::Interface {
                    ident: variant_ident.clone(),
                    fields: variant_fields,
                });
                variants.push(variant_ident);
            }

            self.items
                .push(Item::Alias(ident.clone(), TypeIdent::Union(variants)));
        } else {
            self.items.push(Item::Interface {
                ident: ident.clone(),
                fields: non_union_fields,
            })
        }

        Ok(ident)
    }

    fn visit_enum(&mut self, schema: EnumSchema) -> Result<TypeIdent> {
        let ident = TypeIdent::from_enum(schema)?;

        if self.visited.contains(&ident) {
            return Ok(ident);
        }
        self.visited.insert(ident.clone());

        let mut variants = Vec::new();
        for enumerant in schema.get_enumerants()? {
            let value = enumerant_value(&enumerant)?.to_string();
            variants.push(TypeIdent::Literal(value));
        }

        if self.opts.on_enumerant_not_in_schema == OnEnumerantNotInSchema::UseNumber {
            variants.push(TypeIdent::Number);
        }

        let variants = TypeIdent::Union(variants);

        self.items.push(Item::Alias(ident.clone(), variants));

        Ok(ident)
    }
}

impl Item {
    fn write_to(&self, lines: &mut Vec<String>) {
        match self {
            Item::Interface {
                ident: name,
                fields,
            } => {
                lines.push(format!("interface {name} {{"));
                for (name, value) in fields {
                    lines.push(format!("    {name}: {value};"));
                }
                lines.push("}\n".into());
            }
            Item::Alias(name, value) => {
                lines.push(format!("type {name} = {value};\n"));
            }
        }
    }
}

impl TypeIdent {
    fn for_data(format: DataFormat) -> TypeIdent {
        match format {
            DataFormat::Hex => TypeIdent::HexData,
            DataFormat::Base64 => TypeIdent::Base64Data,
        }
    }

    fn from_struct(schema: StructSchema) -> Result<Self> {
        let ident = Self::capnp_display_name_to_ident(schema.display_name()?)?;
        Ok(Self::Struct(ident))
    }

    fn from_union_variant(schema: StructSchema, variant: Field) -> Result<Self> {
        let base = Self::capnp_display_name_to_ident(schema.display_name()?)?;
        let mut suffix = variant.name()?.chars();
        let suffix_first = suffix
            .next()
            .expect("field name non-empty")
            .to_ascii_uppercase();
        let suffix_rest = suffix.collect::<String>();
        let ident = format!("{base}{suffix_first}{suffix_rest}");
        Ok(Self::Struct(ident))
    }

    fn from_enum(schema: EnumSchema) -> Result<Self> {
        let ident = Self::capnp_display_name_to_ident(schema.display_name()?)?;
        Ok(Self::Enum(ident))
    }

    fn capnp_display_name_to_ident(name: &str) -> Result<String> {
        if !name.is_ascii() {
            return Err(Error::unimplemented(
                "non-ascii names are unsupported".into(),
            ));
        }

        let mut ident = String::new();
        let mut uppercase_next = true;
        for c in name.chars() {
            if c.is_alphanumeric() {
                if uppercase_next {
                    ident.push(c.to_ascii_uppercase());
                    uppercase_next = false;
                } else {
                    ident.push(c);
                }
            } else {
                uppercase_next = true;
            }
        }

        Ok(ident)
    }
}

impl fmt::Display for TypeIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeIdent::Null => write!(f, "null"),
            TypeIdent::Bool => write!(f, "boolean"),
            TypeIdent::Number => write!(f, "number"),
            TypeIdent::String => write!(f, "string"),
            TypeIdent::Base64Data => write!(f, "Base64Data"),
            TypeIdent::HexData => write!(f, "HexData"),
            TypeIdent::Enum(name) => write!(f, "{}", name),
            TypeIdent::Struct(name) => write!(f, "{}", name),
            TypeIdent::Literal(lit) => write!(f, "{}", lit),
            TypeIdent::Union(variants) => {
                for (i, variant) in variants.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", variant)?;
                }
                Ok(())
            }
            TypeIdent::ArrayOf(elem_type) => write!(f, "Array<{elem_type}>"),
        }
    }
}
