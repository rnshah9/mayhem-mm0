// This module may become a plugin in the future, but for now let's avoid the complexity
// of dynamic loading.

//! Compiler tactic for the metamath C language.
//!
//! See [`mmc.md`] for information on the MMC format.
//!
//! [`mmc.md`]: https://github.com/digama0/mm0/blob/master/mm0-rs/mmc.md
pub mod types;
pub mod parser;
pub mod predef;
pub mod nameck;
pub mod typeck;

use std::collections::hash_map::HashMap;
use crate::util::FileSpan;
use crate::elab::{
  Result, Elaborator, ElabError,
  environment::{AtomID, Remap, Remapper},
  lisp::LispVal};
use {types::Keyword, parser::Parser, nameck::Entity, predef::PredefMap, typeck::TypeChecker};

impl Remap for Keyword {
  type Target = Self;
  fn remap(&self, _: &mut Remapper) -> Self { *self }
}

impl Remap for Entity {
  type Target = Self;
  fn remap(&self, _: &mut Remapper) -> Self { self.clone() }
}

impl<A: Remap> Remap for PredefMap<A> {
  type Target = PredefMap<A::Target>;
  fn remap(&self, r: &mut Remapper) -> Self::Target { self.map(|x| x.remap(r)) }
}

/// The MMC compiler, which contains local state for the functions that have been
/// loaded and typechecked thus far.
#[derive(DeepSizeOf)]
pub struct Compiler {
  /// The map of atoms for MMC keywords. (This depends on the environment because
  /// it gets remapped per file.)
  keywords: HashMap<AtomID, Keyword>,
  /// The map of atoms for defined entities (operations and types).
  names: HashMap<AtomID, Entity>,
  /// The map from [`Predef`](predef::Predef) to atoms, used for constructing proofs and referencing
  /// compiler lemmas.
  predef: PredefMap<AtomID>,
  /// A prefix to place on autogenerated names.
  prefix: Vec<u8>,
}

impl std::fmt::Debug for Compiler {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "#<mmc-compiler>")
  }
}

impl Remap for Compiler {
  type Target = Self;
  fn remap(&self, r: &mut Remapper) -> Self {
    Compiler {
      keywords: self.keywords.remap(r),
      names: self.names.remap(r),
      predef: self.predef.remap(r),
      prefix: self.prefix.clone(),
    }
  }
}

impl Compiler {
  /// Create a new [`Compiler`] object. This mutates the elaborator because
  /// it needs to allocate atoms for MMC keywords.
  pub fn new(e: &mut Elaborator) -> Compiler {
    Compiler {
      keywords: e.env.make_keywords(),
      names: Compiler::make_names(&mut e.env),
      predef: PredefMap::new(|_, s| e.env.get_atom(s.as_bytes())),
      prefix: "_mmc_".into(),
    }
  }

  /// Add the given MMC text (as a list of lisp literals) to the compiler state,
  /// performing typehecking but not code generation. This can be called multiple
  /// times to add multiple functions, but each lisp literal is already a list of
  /// top level items that are typechecked as a unit.
  pub fn add(&mut self, elab: &mut Elaborator, fsp: FileSpan, it: impl Iterator<Item=LispVal>) -> Result<()> {
    let mut p = Parser {fe: elab.format_env(), kw: &self.keywords, fsp};
    let mut ast = vec![];
    for e in it {
      if let Some(fsp) = e.fspan() {p.fsp = fsp}
      p.parse_ast(&mut ast, &e)?;
    }
    let fsp = p.fsp;
    for a in &ast { self.nameck(&fsp, a)? }
    let mut tc = TypeChecker::new(self, elab, fsp);
    for item in ast { tc.typeck(&item)? }
    Ok(())
  }

  /// Once we are done adding functions, this function performs final linking to produce an executable.
  #[allow(clippy::unused_self)]
  pub fn finish(&mut self, _elab: &mut Elaborator, _fsp: &FileSpan, _a1: AtomID, _a2: AtomID) -> Result<()> {
    Ok(())
  }

  /// Main entry point to the compiler. Does basic parsing and forwards to
  /// [`add`](Self::add) and [`finish`](Self::finish).
  pub fn call(&mut self, elab: &mut Elaborator, fsp: FileSpan, args: Vec<LispVal>) -> Result<LispVal> {
    let mut it = args.into_iter();
    let e = it.next().expect("expected 1 argument");
    match e.as_atom().and_then(|a| self.keywords.get(&a)) {
      Some(Keyword::Add) => {
        self.add(elab, fsp, it)?;
        Ok(LispVal::undef())
      }
      Some(Keyword::Finish) => {
        let a1 = it.next().and_then(|e| e.as_atom()).ok_or_else(||
          ElabError::new_e(fsp.span, "mmc-finish: syntax error"))?;
        let a2 = it.next().and_then(|e| e.as_atom()).ok_or_else(||
          ElabError::new_e(fsp.span, "mmc-finish: syntax error"))?;
        self.add(elab, fsp.clone(), it)?;
        self.finish(elab, &fsp, a1, a2)?;
        Ok(LispVal::undef())
      }
      _ => Err(ElabError::new_e(fsp.span,
        format!("mmc-compiler: unknown subcommand '{}'", elab.print(&e))))
    }
  }
}