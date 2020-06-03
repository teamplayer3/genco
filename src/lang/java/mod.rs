//! Specialization for Java code generation.

mod modifier;
mod utils;

pub use self::modifier::Modifier;
pub use self::utils::BlockComment;

use crate as genco;
use crate::{quote, Cons, Formatter, Lang, LangItem};
use std::collections::{BTreeSet, HashMap};
use std::fmt;

/// Tokens container specialized for Java.
pub type Tokens<'el> = crate::Tokens<'el, Java>;

impl_type_basics!(Java, TypeEnum<'a>, TypeTrait, TypeBox, TypeArgs, {Primitive, Void, Type, Optional, Local});

/// Trait implemented by all types
pub trait TypeTrait: 'static + fmt::Debug + LangItem<Java> {
    /// Coerce trait into an enum that can be used for type-specific operations
    fn as_enum(&self) -> TypeEnum<'_>;

    /// Get package type belongs to.
    fn name(&self) -> &str;

    /// Get package type belongs to.
    fn package(&self) -> Option<&str> {
        None
    }

    /// Get generic arguments associated with type.
    fn arguments(&self) -> Option<&[TypeBox]> {
        None
    }

    /// Process which kinds of imports to deal with.
    fn type_imports(&self, _: &mut BTreeSet<(Cons<'static>, Cons<'static>)>) {}
}

static JAVA_LANG: &'static str = "java.lang";
static SEP: &'static str = ".";

/// Short primitive type.
pub const SHORT: Primitive = Primitive {
    primitive: "short",
    boxed: "Short",
};

/// Integer primitive type.
pub const INTEGER: Primitive = Primitive {
    primitive: "int",
    boxed: "Integer",
};

/// Long primitive type.
pub const LONG: Primitive = Primitive {
    primitive: "long",
    boxed: "Long",
};

/// Float primitive type.
pub const FLOAT: Primitive = Primitive {
    primitive: "float",
    boxed: "Float",
};

/// Double primitive type.
pub const DOUBLE: Primitive = Primitive {
    primitive: "double",
    boxed: "Double",
};

/// Char primitive type.
pub const CHAR: Primitive = Primitive {
    primitive: "char",
    boxed: "Character",
};

/// Boolean primitive type.
pub const BOOLEAN: Primitive = Primitive {
    primitive: "boolean",
    boxed: "Boolean",
};

/// Byte primitive type.
pub const BYTE: Primitive = Primitive {
    primitive: "byte",
    boxed: "Byte",
};

/// Void type.
pub const VOID: Void = Void(());

/// Configuration for Java formatting.
#[derive(Debug)]
pub struct Config {
    /// Package to use.
    package: Option<Cons<'static>>,

    /// Types which has been imported into the local namespace.
    imported: HashMap<String, String>,
}

impl Config {
    /// Configure package to use.
    pub fn with_package(self, package: impl Into<Cons<'static>>) -> Self {
        Self {
            package: Some(package.into()),
            ..self
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            package: Default::default(),
            imported: Default::default(),
        }
    }
}

/// A class.
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Type {
    /// Package of the class.
    package: Cons<'static>,
    /// Name  of class.
    name: Cons<'static>,
    /// Path of class when nested.
    path: Vec<Cons<'static>>,
    /// Arguments of the class.
    arguments: Vec<TypeBox>,
}

impl Type {
    /// Extend the type with a nested path.
    ///
    /// This discards any arguments associated with it.
    pub fn path<P: Into<Cons<'static>>>(self, part: P) -> Self {
        let mut path = self.path;
        path.push(part.into());

        Self {
            package: self.package,
            name: self.name,
            path: path,
            arguments: vec![],
        }
    }

    /// Add arguments to the given variable.
    ///
    /// Only applies to classes, any other will return the same value.
    pub fn with_arguments(self, args: impl TypeArgs) -> Self {
        Self {
            package: self.package,
            name: self.name,
            path: self.path,
            arguments: args.into_args(),
        }
    }

    /// Get the raw type.
    ///
    /// A raw type is one without generic arguments.
    pub fn as_raw(self) -> Self {
        Self {
            package: self.package,
            name: self.name,
            path: self.path,
            arguments: vec![],
        }
    }

    /// Check if type is generic.
    pub fn is_generic(&self) -> bool {
        !self.arguments.is_empty()
    }
}

impl LangItem<Java> for Type {
    fn format(&self, out: &mut Formatter, config: &mut Config, level: usize) -> fmt::Result {
        {
            let file_package = config.package.as_ref().map(|p| p.as_ref());
            let imported = config.imported.get(self.name.as_ref()).map(String::as_str);
            let pkg = Some(self.package.as_ref());

            if self.package.as_ref() != JAVA_LANG && imported != pkg && file_package != pkg {
                out.write_str(self.package.as_ref())?;
                out.write_str(SEP)?;
            }
        }

        {
            out.write_str(self.name.as_ref())?;

            let mut it = self.path.iter();

            while let Some(n) = it.next() {
                out.write_str(".")?;
                out.write_str(n.as_ref())?;
            }
        }

        if !self.arguments.is_empty() {
            out.write_str("<")?;

            let mut it = self.arguments.iter().peekable();

            while let Some(argument) = it.next() {
                argument.format(out, config, level + 1usize)?;

                if it.peek().is_some() {
                    out.write_str(", ")?;
                }
            }

            out.write_str(">")?;
        }

        Ok(())
    }

    fn as_import(&self) -> Option<&dyn TypeTrait> {
        Some(self)
    }
}

impl TypeTrait for Type {
    fn as_enum(&self) -> TypeEnum<'_> {
        TypeEnum::Type(self)
    }

    fn name(&self) -> &str {
        &*self.name
    }

    fn package(&self) -> Option<&str> {
        Some(&*self.package)
    }

    fn arguments(&self) -> Option<&[TypeBox]> {
        Some(&self.arguments)
    }

    fn type_imports(&self, modules: &mut BTreeSet<(Cons<'static>, Cons<'static>)>) {
        for argument in &self.arguments {
            if let TypeEnum::Type(ty) = argument.as_enum() {
                ty.type_imports(modules);
            }
        }

        modules.insert((self.package.clone(), self.name.clone()));
    }
}

/// The void type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Void(());

impl TypeTrait for Void {
    fn as_enum(&self) -> TypeEnum<'_> {
        TypeEnum::Void(self)
    }

    fn name(&self) -> &str {
        "void"
    }
}

impl LangItem<Java> for Void {
    fn format(&self, out: &mut Formatter, _: &mut Config, level: usize) -> fmt::Result {
        if level > 0 {
            out.write_str("Void")
        } else {
            out.write_str("void")
        }
    }
}

/// A primitive type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Primitive {
    /// The boxed variant of the primitive type.
    boxed: &'static str,
    /// The primitive-primitive type.
    primitive: &'static str,
}

impl Primitive {
    /// Get a boxed version of a primitive type.
    pub fn into_boxed(self) -> Type {
        Type {
            package: Cons::Borrowed(JAVA_LANG),
            name: Cons::Borrowed(self.boxed),
            path: vec![],
            arguments: vec![],
        }
    }
}

impl LangItem<Java> for Primitive {
    fn format(&self, out: &mut Formatter, _: &mut Config, level: usize) -> fmt::Result {
        if level > 0 {
            out.write_str(self.boxed)
        } else {
            out.write_str(self.primitive)
        }
    }
}

impl TypeTrait for Primitive {
    fn name(&self) -> &str {
        self.primitive
    }

    fn as_enum(&self) -> TypeEnum<'_> {
        TypeEnum::Primitive(self)
    }

    fn package(&self) -> Option<&str> {
        Some(JAVA_LANG)
    }
}

/// A local name with no specific qualification.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Local {
    /// Name of class.
    name: Cons<'static>,
}

impl TypeTrait for Local {
    fn as_enum(&self) -> TypeEnum<'_> {
        TypeEnum::Local(self)
    }

    fn name(&self) -> &str {
        &*self.name
    }
}

impl LangItem<Java> for Local {
    fn format(&self, out: &mut Formatter, _: &mut Config, _: usize) -> fmt::Result {
        out.write_str(&*self.name)
    }

    fn as_import(&self) -> Option<&dyn TypeTrait> {
        Some(self)
    }
}

/// An optional type.
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Optional {
    /// The type that is optional.
    pub value: TypeBox,
    /// The complete optional field type, including wrapper.
    pub field: TypeBox,
}

impl TypeTrait for Optional {
    fn as_enum(&self) -> TypeEnum<'_> {
        TypeEnum::Optional(self)
    }

    fn name(&self) -> &str {
        self.value.name()
    }

    fn package(&self) -> Option<&str> {
        self.value.package()
    }

    fn arguments(&self) -> Option<&[TypeBox]> {
        self.value.arguments()
    }

    fn type_imports(&self, modules: &mut BTreeSet<(Cons<'static>, Cons<'static>)>) {
        self.value.type_imports(modules);
    }
}

impl Optional {
    /// Get the field type (includes optionality).
    pub fn as_field(self) -> TypeBox {
        self.field.clone()
    }

    /// Get the value type (strips optionality).
    pub fn as_value(self) -> TypeBox {
        self.value.clone()
    }
}

impl LangItem<Java> for Optional {
    fn format(&self, out: &mut Formatter, config: &mut Config, level: usize) -> fmt::Result {
        self.field.format(out, config, level)
    }

    fn as_import(&self) -> Option<&dyn TypeTrait> {
        Some(self)
    }
}

/// Language specialization for Java.
pub struct Java(());

impl Java {
    fn imports<'el>(tokens: &Tokens<'el>, config: &mut Config) -> Option<Tokens<'el>> {
        let mut modules = BTreeSet::new();

        let file_package = config.package.as_ref().map(|p| p.as_ref());

        for custom in tokens.walk_custom() {
            if let Some(ty) = custom.as_import() {
                ty.type_imports(&mut modules);
            }
        }

        if modules.is_empty() {
            return None;
        }

        let mut out = Tokens::new();

        for (package, name) in modules {
            if config.imported.contains_key(&*name) {
                continue;
            }

            if &*package == JAVA_LANG {
                continue;
            }

            if Some(&*package) == file_package.as_deref() {
                continue;
            }

            out.push(quote!(import #(package)#(SEP)#(name);));
            config
                .imported
                .insert(name.to_string(), package.to_string());
        }

        Some(out)
    }
}

impl Lang for Java {
    type Config = Config;
    type Import = dyn TypeTrait;

    fn quote_string(out: &mut Formatter, input: &str) -> fmt::Result {
        use std::fmt::Write as _;

        out.write_char('"')?;

        for c in input.chars() {
            match c {
                '\t' => out.write_str("\\t")?,
                '\u{0007}' => out.write_str("\\b")?,
                '\n' => out.write_str("\\n")?,
                '\r' => out.write_str("\\r")?,
                '\u{0014}' => out.write_str("\\f")?,
                '\'' => out.write_str("\\'")?,
                '"' => out.write_str("\\\"")?,
                '\\' => out.write_str("\\\\")?,
                c => out.write_char(c)?,
            }
        }

        out.write_char('"')?;

        Ok(())
    }

    fn write_file(
        tokens: Tokens<'_>,
        out: &mut Formatter,
        config: &mut Self::Config,
        level: usize,
    ) -> fmt::Result {
        let mut toks = Tokens::new();

        if let Some(ref package) = config.package {
            toks.push(toks!["package ", package.clone(), ";"]);
            toks.line_spacing();
        }

        if let Some(imports) = Self::imports(&tokens, config) {
            toks.push(imports);
            toks.line_spacing();
        }

        toks.extend(tokens);
        toks.format(out, config, level)
    }
}

/// Setup an imported element.
pub fn imported<P: Into<Cons<'static>>, N: Into<Cons<'static>>>(package: P, name: N) -> Type {
    Type {
        package: package.into(),
        name: name.into(),
        path: vec![],
        arguments: vec![],
    }
}

/// Setup a local element from borrowed components.
pub fn local<'el, N: Into<Cons<'static>>>(name: N) -> Local {
    Local { name: name.into() }
}

/// Setup an optional type.
pub fn optional<'el, I: Into<TypeBox>, F: Into<TypeBox>>(value: I, field: F) -> Optional {
    Optional {
        value: value.into(),
        field: field.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as genco;
    use crate::{quote, Java, Quoted, Tokens};

    #[test]
    fn test_string() {
        let mut toks: Tokens<Java> = Tokens::new();
        toks.append("hello \n world".quoted());
        assert_eq!("\"hello \\n world\"", toks.to_string().unwrap().as_str());
    }

    #[test]
    fn test_imported() {
        let integer = imported("java.lang", "Integer");
        let a = imported("java.io", "A");
        let b = imported("java.io", "B");
        let ob = imported("java.util", "B");
        let ob_a = ob.clone().with_arguments(a.clone());

        let toks = quote!(#integer #a #b #ob #ob_a);

        assert_eq!(
            Ok("import java.io.A;\nimport java.io.B;\n\nInteger A B java.util.B java.util.B<A>\n",),
            toks.to_file_string().as_ref().map(|s| s.as_str())
        );
    }
}