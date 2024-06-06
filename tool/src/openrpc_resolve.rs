use core::fmt;
use std::collections::BTreeMap;

use itertools::Itertools;

use crate::openrpc_types::{
    Components, ContentDescriptor, Error, ExamplePairing, ExternalDocumentation, Method,
    ParamStructure, ReferenceOr, Server, SpecificationExtensions, Tag,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedMethod {
    /// > REQUIRED.
    /// > The cannonical name for the method.
    /// > The name MUST be unique within the methods array.
    pub name: String,
    /// > A list of tags for API documentation control.
    /// > Tags can be used for logical grouping of methods by resources or any other qualifier.
    pub tags: Option<Vec<Tag>>,
    /// > A short summary of what the method does.
    pub summary: Option<String>,
    /// > A verbose explanation of the method behavior.
    /// > GitHub Flavored Markdown syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// > Additional external documentation for this method.
    pub external_docs: Option<ExternalDocumentation>,
    /// > REQUIRED.
    /// > A list of parameters that are applicable for this method.
    /// > The list MUST NOT include duplicated parameters and therefore require name to be unique.
    /// > The list can use the Reference Object to link to parameters that are defined by the Content Descriptor Object.
    /// > All optional params (content descriptor objects with “required”: false) MUST be positioned after all required params in the list.
    pub params: Vec<ContentDescriptor>,
    /// > The description of the result returned by the method.
    /// > If defined, it MUST be a Content Descriptor or Reference Object.
    /// > If undefined, the method MUST only be used as a notification.
    pub result: Option<ContentDescriptor>,
    /// > Declares this method to be deprecated.
    /// > Consumers SHOULD refrain from usage of the declared method.
    /// > Default value is `false`.
    pub deprecated: Option<bool>,
    /// > An alternative servers array to service this method.
    /// > If an alternative servers array is specified at the Root level,
    /// > it will be overridden by this value.
    pub servers: Option<Vec<Server>>,
    /// > A list of custom application defined errors that MAY be returned.
    /// > The Errors MUST have unique error codes.
    pub errors: Option<Vec<Error>>,
    pub param_structure: Option<ParamStructure>,
    /// > Array of Example Pairing Objects where each example includes a valid params-to-result Content Descriptor pairing.
    pub examples: Option<Vec<ExamplePairing>>,
    pub extensions: SpecificationExtensions,
}

pub fn methods(
    components: Option<&Components>,
    methods: Vec<ReferenceOr<Method>>,
) -> Result<Vec<ResolvedMethod>, ResolveError> {
    methods
        .into_iter()
        .map(|it| {
            resolve(
                components,
                it,
                "methods",
                // TODO(aatifsyed): there is a bug in the OpenRPC spec, where there are no `methods` components
                |_| None,
            )
            .and_then(|it| self::method(components, it))
            .map_err(ResolveError)
        })
        .collect()
}

fn method(components: Option<&Components>, method: Method) -> Result<ResolvedMethod, String> {
    let Method {
        name,
        tags,
        summary,
        description,
        external_docs,
        params,
        result,
        deprecated,
        servers,
        errors,
        param_structure,
        examples,
        extensions,
    } = method;
    Ok(ResolvedMethod {
        name,
        tags: match tags {
            Some(it) => Some(
                it.into_iter()
                    .map(|it| resolve(components, it, "tags", |it| it.tags.as_ref()))
                    .try_collect()?,
            ),
            None => None,
        },
        summary,
        description,
        external_docs,
        params: params
            .into_iter()
            .map(|it| {
                resolve(components, it, "contentDescriptors", |it| {
                    it.content_descriptors.as_ref()
                })
            })
            .try_collect()?,
        result: match result {
            Some(it) => Some(resolve(components, it, "contentDescriptors", |it| {
                it.content_descriptors.as_ref()
            })?),
            None => None,
        },
        deprecated,
        servers,
        errors: match errors {
            Some(it) => Some(
                it.into_iter()
                    .map(|it| resolve(components, it, "errors", |it| it.errors.as_ref()))
                    .try_collect()?,
            ),
            None => None,
        },
        param_structure,
        // TODO(aatifsyed): this should be a ResolvedExample, but we're not checking that yet.
        examples: match examples {
            Some(it) => Some(
                it.into_iter()
                    .map(|it| {
                        resolve(components, it, "examplePairingObjects", |it| {
                            it.example_pairing_objects.as_ref()
                        })
                    })
                    .try_collect()?,
            ),
            None => None,
        },
        extensions,
    })
}

fn resolve<T: Clone>(
    components: Option<&Components>,
    refr: ReferenceOr<T>,
    path: &str,
    getter: impl Fn(&Components) -> Option<&BTreeMap<String, T>>,
) -> Result<T, String> {
    match refr {
        ReferenceOr::Reference(it) => match it.strip_prefix(&format!("#/components/{}/", path)) {
            Some(key) => match components.map(getter) {
                Some(Some(map)) => match map.get(key) {
                    Some(it) => Ok(it.clone()),
                    None => Err(it),
                },
                Some(None) | None => Err(it),
            },
            None => Err(it),
        },
        ReferenceOr::Item(it) => Ok(it),
    }
}

#[derive(Debug)]
pub struct ResolveError(String);

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("error resolving `$ref`: {}", self.0))
    }
}

impl std::error::Error for ResolveError {}
