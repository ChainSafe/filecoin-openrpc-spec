use std::{
    collections::{BTreeMap, HashSet},
    iter,
};

use either::Either;
use openrpc_types::{resolved, BrokenReference};
use schemars::schema::{
    ArrayValidation, ObjectValidation, Schema, SchemaObject, SingleOrVec, SubschemaValidation,
};

pub fn prune_schemas(document: &mut resolved::OpenRPC) -> Result<(), BrokenReference> {
    let mut alive = HashSet::new();

    for root in document
        .methods
        .iter()
        .flat_map(|it| it.params.iter().chain(it.result.as_ref()))
    {
        mark(
            &mut alive,
            document
                .components
                .as_ref()
                .and_then(|it| it.schemas.as_ref()),
            &root.schema,
        )?;
    }

    // sweep
    if let Some(it) = document
        .components
        .as_mut()
        .and_then(|it| it.schemas.as_mut())
    {
        it.retain(|k, _| alive.contains(k))
    }

    Ok(())
}

fn mark(
    alive: &mut HashSet<String>,
    lookup: Option<&BTreeMap<String, Schema>>,
    schema: &Schema,
) -> Result<(), BrokenReference> {
    match schema {
        Schema::Bool(_) => Ok(()),
        Schema::Object(SchemaObject {
            metadata: _,
            instance_type: _,
            format: _,
            enum_values: _,
            const_value: _,
            subschemas,
            number: _,
            string: _,
            array,
            object,
            reference,
            extensions: _,
        }) => {
            if let Some(SubschemaValidation {
                all_of,
                any_of,
                one_of,
                not,
                if_schema,
                then_schema,
                else_schema,
            }) = subschemas.as_deref()
            {
                for schema in iter::empty()
                    .chain(all_of.iter().flatten())
                    .chain(any_of.iter().flatten())
                    .chain(one_of.iter().flatten())
                    .chain(not.as_deref())
                    .chain(if_schema.as_deref())
                    .chain(then_schema.as_deref())
                    .chain(else_schema.as_deref())
                {
                    mark(alive, lookup, schema)?
                }
            };
            if let Some(ArrayValidation {
                items,
                additional_items,
                max_items: _,
                min_items: _,
                unique_items: _,
                contains,
            }) = array.as_deref()
            {
                for schema in items
                    .iter()
                    .flat_map(iter_single_or_vec)
                    .chain(additional_items.as_deref())
                    .chain(contains.as_deref())
                {
                    mark(alive, lookup, schema)?
                }
            }
            if let Some(ObjectValidation {
                max_properties: _,
                min_properties: _,
                required: _,
                properties,
                pattern_properties,
                additional_properties,
                property_names,
            }) = object.as_deref()
            {
                for schema in properties
                    .values()
                    .chain(pattern_properties.values())
                    .chain(additional_properties.as_deref())
                    .chain(property_names.as_deref())
                {
                    mark(alive, lookup, schema)?
                }
            }
            if let Some(reference) = reference {
                match reference.strip_prefix("#/components/schemas/") {
                    Some(key) => {
                        if !alive.contains(key) {
                            alive.insert(key.to_owned());
                            match lookup.as_ref().and_then(|it| it.get(key)) {
                                Some(child) => mark(alive, lookup, child)?,
                                None => return Err(BrokenReference(reference.clone())),
                            }
                        }
                    }
                    None => return Err(BrokenReference(reference.clone())),
                }
            }
            Ok(())
        }
    }
}

fn iter_single_or_vec<T>(it: &SingleOrVec<T>) -> impl Iterator<Item = &T> {
    match it {
        SingleOrVec::Single(it) => Either::Left(iter::once(&**it)),
        SingleOrVec::Vec(it) => Either::Right(it.iter()),
    }
}
