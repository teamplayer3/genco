//! Specialization for Go code generation.
//!
//! # Examples
//!
//! Basic example:
//!
//! ```rust
//! use genco::prelude::*;
//!
//! # fn main() -> genco::fmt::Result {
//! let toks: js::Tokens = quote! {
//!     function foo(v) {
//!         return v + ", World";
//!     }
//!
//!     foo("Hello");
//! };
//!
//! assert_eq!(
//!     vec![
//!         "function foo(v) {",
//!         "    return v + \", World\";",
//!         "}",
//!         "",
//!         "foo(\"Hello\");",
//!     ],
//!     toks.to_file_vec()?
//! );
//! # Ok(())
//! # }
//! ```
//!
//! String quoting in JavaScript:
//!
//! ```rust
//! use genco::prelude::*;
//!
//! # fn main() -> genco::fmt::Result {
//! let toks: go::Tokens = quote!(#("hello \n world".quoted()));
//! assert_eq!("\"hello \\n world\"", toks.to_string()?);
//! # Ok(())
//! # }
//! ```

use crate as genco;
use crate::fmt;
use crate::tokens::ItemStr;
use crate::{quote_in, Lang, LangItem};
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// Tokens container specialization for Go.
pub type Tokens = crate::Tokens<Go>;

impl_dynamic_types! { Go =>
    pub trait TypeTrait {
        /// Handle imports for the given type.
        fn type_imports(&self, _: &mut BTreeSet<ItemStr>) {}
    }

    pub trait Args;
    pub struct Any;
    pub enum AnyRef;

    impl TypeTrait for Type {
        fn type_imports(&self, modules: &mut BTreeSet<ItemStr>) {
            if let Some(module) = &self.module {
                modules.insert(module.clone());
            }
        }
    }

    impl TypeTrait for Map {
        fn type_imports(&self, modules: &mut BTreeSet<ItemStr>) {
            self.key.type_imports(modules);
            self.value.type_imports(modules);
        }
    }

    impl TypeTrait for Interface {}

    impl TypeTrait for Array {
        fn type_imports(&self, modules: &mut BTreeSet<ItemStr>) {
            self.inner.type_imports(modules);
        }
    }
}

/// The interface type `interface{}`.
pub const INTERFACE: Interface = Interface(());

const SEP: &str = ".";

/// A Go type.
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Type {
    /// Module of the imported name.
    module: Option<ItemStr>,
    /// Name imported.
    name: ItemStr,
}

impl_lang_item! {
    impl LangItem<Go> for Type {
        fn format(&self, out: &mut fmt::Formatter<'_>, _: &Config, _: &Format) -> fmt::Result {
            if let Some(module) = self.module.as_ref().and_then(|m| m.split("/").last()) {
                out.write_str(module)?;
                out.write_str(SEP)?;
            }

            out.write_str(&self.name)?;
            Ok(())
        }

        fn as_import(&self) -> Option<&dyn TypeTrait> {
            Some(self)
        }
    }
}

/// A map `map[<key>]<value>`.
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Map {
    /// Key of the map.
    key: Any,
    /// Value of the map.
    value: Any,
}

impl_lang_item! {
    impl LangItem<Go> for Map {
        fn format(&self, out: &mut fmt::Formatter<'_>, config: &Config, format: &Format) -> fmt::Result {
            out.write_str("map[")?;
            self.key.format(out, config, format)?;
            out.write_str("]")?;
            self.value.format(out, config, format)?;
            Ok(())
        }

        fn as_import(&self) -> Option<&dyn TypeTrait> {
            Some(self)
        }
    }
}

/// An array `[]<inner>`.
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Array {
    /// Inner value of the array.
    inner: Any,
}

impl_lang_item! {
    impl LangItem<Go> for Array {
        fn format(&self, out: &mut fmt::Formatter<'_>, config: &Config, format: &Format) -> fmt::Result {
            out.write_str("[")?;
            out.write_str("]")?;
            self.inner.format(out, config, format)?;
            Ok(())
        }

        fn as_import(&self) -> Option<&dyn TypeTrait> {
            Some(self)
        }
    }
}

/// The interface type `interface{}`.
#[derive(Debug, Clone, Copy, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Interface(());

impl_lang_item! {
    impl LangItem<Go> for Interface {
        fn format(&self, out: &mut fmt::Formatter<'_>, _: &Config, _: &Format) -> fmt::Result {
            out.write_str("interface{}")
        }

        fn as_import(&self) -> Option<&dyn TypeTrait> {
            Some(self)
        }
    }
}

/// Format for Go.
#[derive(Debug, Default)]
pub struct Format {}

/// Config data for Go.
#[derive(Debug, Default)]
pub struct Config {
    package: Option<ItemStr>,
}

impl Config {
    /// Configure the specified package.
    pub fn with_package<P: Into<ItemStr>>(self, package: P) -> Self {
        Self {
            package: Some(package.into()),
            ..self
        }
    }
}

/// Language specialization for Go.
pub struct Go(());

impl Go {
    fn imports(out: &mut Tokens, tokens: &Tokens) {
        use crate::ext::QuotedExt as _;

        let mut modules = BTreeSet::new();

        for import in tokens.walk_imports() {
            import.type_imports(&mut modules);
        }

        if modules.is_empty() {
            return;
        }

        for module in modules {
            quote_in!(*out => import #(module.quoted()));
            out.push();
        }

        out.line();
    }
}

impl Lang for Go {
    type Config = Config;
    type Format = Format;
    type Import = dyn TypeTrait;

    fn quote_string(out: &mut fmt::Formatter<'_>, input: &str) -> fmt::Result {
        out.write_char('"')?;

        for c in input.chars() {
            match c {
                '\t' => out.write_str("\\t")?,
                '\n' => out.write_str("\\n")?,
                '\r' => out.write_str("\\r")?,
                '\'' => out.write_str("\\'")?,
                '"' => out.write_str("\\\"")?,
                '\\' => out.write_str("\\\\")?,
                c => out.write_char(c)?,
            };
        }

        out.write_char('"')?;
        Ok(())
    }

    fn format_file(
        tokens: &Tokens,
        out: &mut fmt::Formatter<'_>,
        config: &Self::Config,
    ) -> fmt::Result {
        let mut header = Tokens::new();

        if let Some(package) = &config.package {
            quote_in!(header => package #package);
            header.line();
        }

        Self::imports(&mut header, tokens);
        let format = Format::default();
        header.format(out, config, &format)?;
        tokens.format(out, config, &format)?;
        Ok(())
    }
}

/// Setup an imported element.
///
/// # Examples
///
/// ```rust
/// use genco::prelude::*;
///
/// # fn main() -> genco::fmt::Result {
/// let ty = go::imported("foo", "Debug");
///
/// let toks = quote! {
///     #ty
/// };
///
/// assert_eq!(
///     vec![
///        "import \"foo\"",
///        "",
///        "foo.Debug",
///     ],
///     toks.to_file_vec()?
/// );
/// # Ok(())
/// # }
/// ```
pub fn imported<M, N>(module: M, name: N) -> Type
where
    M: Into<ItemStr>,
    N: Into<ItemStr>,
{
    Type {
        module: Some(module.into()),
        name: name.into(),
    }
}

/// Setup a local element.
///
/// # Examples
///
/// ```rust
/// use genco::prelude::*;
///
/// # fn main() -> genco::fmt::Result {
/// let toks = quote!(#(go::local("MyType")));
/// assert_eq!(vec!["MyType"], toks.to_file_vec()?);
/// # Ok(())
/// # }
/// ```
pub fn local<N>(name: N) -> Type
where
    N: Into<ItemStr>,
{
    Type {
        module: None,
        name: name.into(),
    }
}

/// Setup a map.
///
/// # Examples
///
/// ```rust
/// use genco::prelude::*;
///
/// # fn main() -> genco::fmt::Result {
/// let ty = go::map(go::imported("foo", "Debug"), go::INTERFACE);
///
/// let toks = quote! {
///     #ty
/// };
///
/// assert_eq!(
///     vec![
///         "import \"foo\"",
///         "",
///         "map[foo.Debug]interface{}",
///     ],
///     toks.to_file_vec()?
/// );
/// # Ok(())
/// # }
/// ```
pub fn map<K, V>(key: K, value: V) -> Map
where
    K: Into<Any>,
    V: Into<Any>,
{
    Map {
        key: key.into(),
        value: value.into(),
    }
}

/// Setup an array.
///
/// # Examples
///
/// ```rust
/// use genco::prelude::*;
///
/// # fn main() -> genco::fmt::Result {
/// let import = go::array(go::imported("foo", "Debug"));
///
/// let toks = quote!(#import);
///
/// assert_eq!(
///     vec![
///         "import \"foo\"",
///         "",
///         "[]foo.Debug",
///     ],
///     toks.to_file_vec()?
/// );
/// # Ok(())
/// # }
/// ```
pub fn array<I>(inner: I) -> Array
where
    I: Into<Any>,
{
    Array {
        inner: inner.into(),
    }
}
