// use std::cell::OnceCell;
// use std::marker::PhantomData;
//
// use crate::util;
//
// // ───── ScriptName newtype/typestate wrapper ───────────────────── //
// /// Newtype wrapper for script names w/ cached canonicalization
// /// and typestate to track where it came from
// #[derive(Clone, Debug, Eq)]
// pub struct ScriptName<P: Provenance> {
//     pub name: String,
//     _canonicalized: OnceCell<String>,
//     __provenance: PhantomData<P>,
// }
//
// impl<P: Provenance> ScriptName<P> {
//     pub fn new(name: String) -> Self {
//         Self {
//             name,
//             _canonicalized: Default::default(),
//             __provenance: Default::default(),
//         }
//     }
//
//     pub fn canonical(&self) -> &str {
//         self._canonicalized
//             .get_or_init(|| util::canonicalize_crate_name(&self.name))
//     }
// }
//
// impl<P: Provenance> From<String> for ScriptName<P> {
//     fn from(value: String) -> Self {
//         Self::new(value)
//     }
// }
//
// /// Comparing script names of different provenances is totally fine
// impl<P1: Provenance, P2: Provenance> PartialEq<ScriptName<P2>>
//     for ScriptName<P1>
// {
//     fn eq(&self, other: &ScriptName<P2>) -> bool {
//         self.canonical() == other.canonical()
//     }
// }
//
// // ───── Provenance (e.g., where the name came from) ─────
// pub trait Provenance {}
// #[derive(Clone, Debug, PartialEq, Eq)]
// pub struct FromUserInput;
// impl Provenance for FromUserInput {}
//
// #[derive(Clone, Debug, PartialEq, Eq)]
// pub struct FromManifest;
// impl Provenance for FromManifest {}
