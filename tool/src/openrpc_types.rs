//! A transcription of types from the [`OpenRPC` Specification](https://spec.open-rpc.org/).
//!
//! This library does NOT perform more complicated validation of the spec, including:
//! - unique method names
//! - unique error codes
//! - reference idents
//!
//! `Link` objects are not currently supported.
//!
//! > When quoted, the specification will appear as blockquoted text, like so.

use schemars::schema::Schema;
use semver::{BuildMetadata, Prerelease, Version};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use url::Url;

/// The version of the OpenRPC specification that this library was written against.
pub const OPEN_RPC_SPECIFICATION_VERSION: Version = Version {
    major: 1,
    minor: 3,
    patch: 2,
    pre: Prerelease::EMPTY,
    build: BuildMetadata::EMPTY,
};

/// > This is the root object of the OpenRPC document.
/// > The contents of this object represent a whole OpenRPC document.
/// > How this object is constructed or stored is outside the scope of the OpenRPC Specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRPC {
    /// > REQUIRED.
    /// > This string MUST be the semantic version number of the OpenRPC Specification version that the OpenRPC document uses.
    /// > The openrpc field SHOULD be used by tooling specifications and clients to interpret the OpenRPC document.
    /// > This is not related to the API info.version string.
    pub openrpc: Version,
    /// > REQUIRED.
    /// > Provides metadata about the API.
    /// > The metadata MAY be used by tooling as required.
    pub info: Info,
    /// > An array of Server Objects,
    /// > which provide connectivity information to a target server.
    /// > If the servers property is not provided, or is an empty array,
    /// > the default value would be a Server Object with a url value of `localhost`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servers: Option<Vec<Server>>,
    /// > REQUIRED.
    /// > The available methods for the API.
    /// > While it is required, the array may be empty (to handle security filtering, for example).
    pub methods: Vec<ReferenceOr<Method>>,
    /// > An element to hold various schemas for the specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
    /// > Additional external documentation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocumentation>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

impl Default for OpenRPC {
    fn default() -> Self {
        Self {
            openrpc: OPEN_RPC_SPECIFICATION_VERSION,
            info: Default::default(),
            servers: Default::default(),
            methods: Default::default(),
            components: Default::default(),
            external_docs: Default::default(),
            extensions: Default::default(),
        }
    }
}

/// > The object provides metadata about the API.
/// > The metadata MAY be used by the clients if needed,
/// > and MAY be presented in editing or documentation generation tools for convenience.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    /// > REQUIRED.
    /// > The title of the application.
    pub title: String,
    /// > A verbose description of the application.
    /// > GitHub Flavored Markdown syntax MAY be used for rich text representation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// > A URL to the Terms of Service for the API.
    /// > MUST be in the format of a URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_of_service: Option<Url>,
    /// > The contact information for the exposed API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Contact>,
    /// > The license information for the exposed API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    /// > REQUIRED.
    /// > The version of the OpenRPC document
    /// > (which is distinct from the OpenRPC Specification version or the API implementation version).
    pub version: String,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

/// > Contact information for the exposed API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Contact {
    /// > The identifying name of the contact person/organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// > The URL pointing to the contact information.
    /// > MUST be in the format of a URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,
    /// > The email address of the contact person/organization.
    /// > MUST be in the format of an email address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

/// > License information for the exposed API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct License {
    /// > REQUIRED.
    /// > The license name used for the API.
    pub name: String,
    /// > A URL to the license used for the API.
    /// > MUST be in the format of a URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

/// > An object representing a Server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Server {
    /// > REQUIRED.
    /// > A name to be used as the cannonical name for the server.
    pub name: String,
    /// > REQUIRED.
    /// > A URL to the target host.
    /// > This URL supports Server Variables and MAY be relative,
    /// > to indicate that the host location is relative to the location where the OpenRPC document is being served.
    /// > Server Variables are passed into the Runtime Expression to produce a server URL.
    pub url: String,
    /// > A short summary of what the server is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// > An optional string describing the host designated by the URL.
    /// > GitHub Flavored Markdown syntax MAY be used for rich text representation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// > A map between a variable name and its value.
    /// > The value is passed into the Runtime Expression to produce a server URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<BTreeMap<String, ServerVariable>>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}
/// > An object representing a Server Variable for server URL template substitution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ServerVariable {
    /// > An enumeration of string values to be used if the substitution options are from a limited set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<String>>,
    /// > REQUIRED.
    /// > The default value to use for substitution,
    /// > which SHALL be sent if an alternate value is not supplied.
    /// > Note this behavior is different than the Schema Object’s treatment of default values,
    /// > because in those cases parameter values are optional.
    pub default: String,
    /// > An optional description for the server variable. GitHub Flavored Markdown syntax MAY be used for rich text representation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

/// > Describes the interface for the given method name.
/// > The method name is used as the method field of the JSON-RPC body.
/// > It therefore MUST be unique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Method {
    /// > REQUIRED.
    /// > The cannonical name for the method.
    /// > The name MUST be unique within the methods array.
    pub name: String,
    /// > A list of tags for API documentation control.
    /// > Tags can be used for logical grouping of methods by resources or any other qualifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ReferenceOr<Tag>>>,
    /// > A short summary of what the method does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// > A verbose explanation of the method behavior.
    /// > GitHub Flavored Markdown syntax MAY be used for rich text representation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// > Additional external documentation for this method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocumentation>,
    /// > REQUIRED.
    /// > A list of parameters that are applicable for this method.
    /// > The list MUST NOT include duplicated parameters and therefore require name to be unique.
    /// > The list can use the Reference Object to link to parameters that are defined by the Content Descriptor Object.
    /// > All optional params (content descriptor objects with “required”: false) MUST be positioned after all required params in the list.
    pub params: Vec<ReferenceOr<ContentDescriptor>>,
    /// > The description of the result returned by the method.
    /// > If defined, it MUST be a Content Descriptor or Reference Object.
    /// > If undefined, the method MUST only be used as a notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ReferenceOr<ContentDescriptor>>,
    /// > Declares this method to be deprecated.
    /// > Consumers SHOULD refrain from usage of the declared method.
    /// > Default value is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    /// > An alternative servers array to service this method.
    /// > If an alternative servers array is specified at the Root level,
    /// > it will be overridden by this value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servers: Option<Vec<Server>>,
    /// > A list of custom application defined errors that MAY be returned.
    /// > The Errors MUST have unique error codes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ReferenceOr<Error>>>,
    // /// > A list of possible links from this method call.
    // pub links: Option<Vec<ReferenceOr<Link>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param_structure: Option<ParamStructure>,
    /// > Array of Example Pairing Objects where each example includes a valid params-to-result Content Descriptor pairing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<ReferenceOr<ExamplePairing>>>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentDescriptor {
    /// > REQUIRED.
    /// > Name of the content that is being described.
    /// > If the content described is a method parameter assignable by-name, this field SHALL define the parameter’s key (ie name).
    pub name: String,
    /// > A short summary of the content that is being described.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// > A verbose explanation of the content descriptor behavior.
    /// > GitHub Flavored Markdown syntax MAY be used for rich text representation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// > Determines if the content is a required field.
    /// > Default value is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// > REQUIRED.
    /// > Schema that describes the content.
    ///
    /// > The Schema Object allows the definition of input and output data types.
    /// > The Schema Objects MUST follow the specifications outline in the JSON Schema Specification 7 Alternatively,
    /// > any time a Schema Object can be used, a Reference Object can be used in its place.
    /// > This allows referencing definitions instead of defining them inline.
    ///
    /// > This object MAY be extended with Specification Extensions.
    pub schema: Schema,
    /// > Specifies that the content is deprecated and SHOULD be transitioned out of usage.
    /// > Default value is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

impl Default for ContentDescriptor {
    fn default() -> Self {
        Self {
            name: Default::default(),
            summary: Default::default(),
            description: Default::default(),
            required: Default::default(),
            schema: Schema::Bool(false),
            deprecated: Default::default(),
            extensions: Default::default(),
        }
    }
}

/// > The Example Pairing object consists of a set of example params and result.
/// > The result is what you can expect from the JSON-RPC service given the exact params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ExamplePairing {
    /// > REQUIRED Name for the example pairing.
    pub name: String,
    /// > A verbose explanation of the example pairing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// > Short description for the example pairing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// > REQUIRED Example parameters.
    pub params: Vec<ReferenceOr<Example>>,
    /// > Example result.
    /// > When undefined, the example pairing represents usage of the method as a notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ReferenceOr<Example>>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

/// > The Example object is an object that defines an example that is intended to match the schema of a given Content Descriptor.
/// >
/// > In all cases, the example value is expected to be compatible with the type schema of its associated value.
/// > Tooling implementations MAY choose to validate compatibility automatically,
/// > and reject the example value(s) if incompatible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    /// Cannonical name of the example.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Short description for the example.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// > A verbose explanation of the example.
    /// > GitHub Flavored Markdown syntax MAY be used for rich text representation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// > Embedded literal example.
    /// > The value field and externalValue field are mutually exclusive.
    /// > To represent examples of media types that cannot naturally represented in JSON,
    /// > use a string value to contain the example, escaping where necessary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// > A URL that points to the literal example.
    /// > This provides the capability to reference examples that cannot easily be included in JSON documents.
    /// > The value field and externalValue field are mutually exclusive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_value: Option<String>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

// pub struct Link {} // TODO

/// > Defines an application level error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Error {
    /// > REQUIRED.
    /// > A Number that indicates the error type that occurred.
    /// > This MUST be an integer.
    /// > The error codes from and including -32768 to -32000 are reserved for pre-defined errors.
    /// > These pre-defined errors SHOULD be assumed to be returned from any JSON-RPC api.
    pub code: i64,
    /// > REQUIRED.
    /// > A String providing a short description of the error.
    /// > The message SHOULD be limited to a concise single sentence.
    pub message: String,
    /// > A Primitive or Structured value that contains additional information about the error.
    /// > This may be omitted.
    /// > The value of this member is defined by the Server (e.g. detailed error information, nested errors etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// > Holds a set of reusable objects for different aspects of the OpenRPC.
/// > All objects defined within the components object will have no effect on the
/// > API unless they are explicitly referenced from properties outside the components object.
/// > All the fixed fields declared above are objects that MUST use keys that match the regular expression: ^[a-zA-Z0-9\.\-_]+$
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]

pub struct Components {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_descriptors: Option<BTreeMap<String, ContentDescriptor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<BTreeMap<String, Schema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<BTreeMap<String, Example>>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub links: Option<BTreeMap<String, Link>>, // TODO
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<BTreeMap<String, Error>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_pairing_objects: Option<BTreeMap<String, ExamplePairing>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<BTreeMap<String, Tag>>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

/// > Adds metadata to a single tag that is used by the Method Object.
/// > It is not mandatory to have a Tag Object per tag defined in the Method Object instances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    /// > REQUIRED.
    /// > The name of the tag.
    pub name: String,
    /// > A short summary of the tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// > A verbose explanation for the tag.
    /// > GitHub Flavored Markdown syntax MAY be used for rich text representation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// > Additional external documentation for this tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocumentation>,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

/// > Allows referencing an external resource for extended documentation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalDocumentation {
    /// > A verbose explanation of the target documentation.
    /// > GitHub Flavored Markdown syntax MAY be used for rich text representation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// > The URL for the target documentation.
    /// > Value MUST be in the format of a URL.
    pub url: Url,
    #[serde(flatten)]
    pub extensions: SpecificationExtensions,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceOr<T> {
    /// > A simple object to allow referencing other components in the specification, internally and externally.
    /// > The Reference Object is defined by JSON Schema and follows the same structure, behavior and rules.
    Reference(String),
    Item(T),
}

/// > While the OpenRPC Specification tries to accommodate most use cases,
/// > additional data can be added to extend the specification at certain points.
/// >
/// > The extensions properties are implemented as patterned fields that are always prefixed by "x-".
/// >
/// > The extensions may or may not be supported by the available tooling,
/// > but those may be extended as well to add requested support
/// > (if tools are internal or open-sourced).
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(transparent)]
pub struct SpecificationExtensions(pub BTreeMap<String, Value>);

impl<'de> Deserialize<'de> for SpecificationExtensions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = BTreeMap<String, Value>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map with string keys starting with `x-`")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut ret = Self::Value::default();
                loop {
                    match map.next_key::<String>() {
                        Err(_) => (),
                        Ok(None) => break,
                        Ok(Some(key)) if key.starts_with("x-") => {
                            let _ = ret.insert(key, map.next_value()?);
                        }
                        Ok(Some(_)) => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(ret)
            }
        }
        deserializer.deserialize_any(Visitor).map(Self)
    }
}

/// > The expected format of the parameters.
/// > As per the JSON-RPC 2.0 specification,
/// > the params of a JSON-RPC request object may be an array, object, or either
/// > (represented as by-position, by-name, and either respectively).
/// > When a method has a paramStructure value of by-name,
/// > callers of the method MUST send a JSON-RPC request object whose params field is an object.
/// > Further, the key names of the params object MUST be the same as the contentDescriptor.names for the given method.
/// > Defaults to "either".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ParamStructure {
    ByName,
    ByPosition,
    #[default]
    Either,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged, expecting = "a reference or an inline item")]
enum _ReferenceOr<T> {
    Reference {
        #[serde(rename = "$ref")]
        reference: String,
    },
    Item(T),
}

impl<T> ReferenceOr<T> {
    pub fn map_item<U>(self, f: impl FnOnce(T) -> U) -> ReferenceOr<U> {
        match self {
            ReferenceOr::Reference(it) => ReferenceOr::Reference(it),
            ReferenceOr::Item(it) => ReferenceOr::Item(f(it)),
        }
    }
}

impl<T: Serialize> Serialize for ReferenceOr<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ReferenceOr::Reference(it) => _ReferenceOr::Reference {
                reference: it.clone(),
            },
            ReferenceOr::Item(it) => _ReferenceOr::Item(it),
        }
        .serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ReferenceOr<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        _ReferenceOr::deserialize(deserializer).map(|it| match it {
            _ReferenceOr::Reference { reference } => ReferenceOr::Reference(reference),
            _ReferenceOr::Item(it) => ReferenceOr::Item(it),
        })
    }
}
