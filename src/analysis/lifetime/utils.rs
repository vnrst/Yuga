/*
    Note - this file is not meant to be understood by anyone but me.

    It is a hopeless mess of random functions, some of which are used elsewhere in the code,
    and some of which are no longer used but I haven't cleaned up.
    There may be plenty of redundancy and inefficiency, and some stuff may (appear to) not make sense.
*/

use rustc_hir::{def_id::DefId, Ty, TyKind, Mutability, ConstContext};
use rustc_hir::ParamName::{Plain};
use rustc_hir::LifetimeName;

use rustc_middle::mir::{Operand, Statement, StatementKind, Rvalue, VarDebugInfo, Place, Local, PlaceElem};
use rustc_middle::ty::TyCtxt;
use rustc_middle::hir::map::Map;
use rustc_middle::mir::Body;

use std::collections::HashMap;
use std::matches;

#[derive(Debug)]
#[derive(Clone)]
#[derive(PartialEq)]
pub enum MyProjection {
    MyDeref,
    MyField(usize)
}

pub fn is_self(ty: &Ty) -> bool {
    if let TyKind::Path(rustc_hir::QPath::Resolved(_, path)) = ty.kind {
        if let rustc_hir::def::Res::SelfTyAlias{..} = path.res {
            return true;
        }
    }
    false
}

pub fn get_first_field(proj: &Vec<MyProjection>) -> Option<usize> {
    let mut field_num: Option<usize> = None;

    for p in proj.iter() {
        if let MyProjection::MyField(field) = p {
            field_num = Some(*field);
            break;
        }
    }
    return field_num;
}

pub fn get_mir_value_from_hir_param<'a>(param: &rustc_hir::Param,
                                        mir_body: &Body<'a>
                                    ) -> Option<Place<'a>> {
    /*
        Map from HIR function parameter to MIR variable
        Weird, but there seems to be no way to do it except to match identifier names
    */

    let mut ret_place : Option<Place> = None;

    if let rustc_hir::PatKind::Binding(_, _, id_name_from_hir, _) = (param).pat.kind {

        for v in &(mir_body.var_debug_info) {

            let VarDebugInfo{name: var_name, value: var_info, ..} = v;

            if *var_name == id_name_from_hir.name {
                if let rustc_middle::mir::VarDebugInfoContents::Place(place) = var_info {
                    ret_place = Some(*place);
                }
            }
        }
    }
    ret_place
}

pub fn get_mir_fn_from_defid<'tcx>(
    tcx: &TyCtxt<'tcx>,
    def_id: DefId,
) -> Option<&'tcx Body<'tcx>> {

    if tcx.is_mir_available(def_id)
        && matches!(
            tcx.hir().body_const_context(def_id.expect_local()),
            None | Some(ConstContext::ConstFn)
        )
    {
        Some(tcx.optimized_mir(def_id))
    } else {
        debug!(
            "Skipping an item {:?}, no MIR available for this item",
            def_id
        );
        None
    }
}

pub fn get_defid_args_from_kind<'a, 'tcx>(kind: &'a TyKind<'tcx>)
                -> (Option<DefId>, Vec<&'a rustc_hir::GenericArg<'tcx>>) {

    let mut ret_def_id: Option<DefId>               = None;
    let mut ret_args: Vec<&rustc_hir::GenericArg>   = Vec::new();

    if let TyKind::Path(
                rustc_hir::QPath::Resolved(_,
                    rustc_hir::Path{
                        res,
                        segments,
                        ..
                    }
                )
            ) = kind {

        match res {
            rustc_hir::def::Res::Def(_, def_id) => {
                ret_def_id = Some(*def_id);
            },
            rustc_hir::def::Res::SelfTyAlias{
                    alias_to: def_id,
                    ..
            } => {
                Some(*def_id);
            }
            _ => ()
        }

        if let Some(
                rustc_hir::PathSegment{
                    args : Some(
                        rustc_hir::GenericArgs{
                            args,
                            ..
                        }
                    ),
                    ..
                }
            ) = segments.last() {

            for arg in &(**args) {
                ret_args.push(arg);
            }
        }
    }

    (ret_def_id, ret_args)
}

pub fn get_nested_defs_from_type<'a>(ty: &'a Ty<'a>) -> Vec<DefId> {

    let sub_types: Vec<&Ty> = get_nested_types_from_type(ty);
    let mut defs: Vec<DefId> = Vec::new();

    for sub_type in sub_types.iter() {
        let (def_id, _) = get_defid_args_from_kind(&sub_type.kind);

        if let Some(def_id) = def_id {
            defs.push(def_id);
        }
    }
    defs
}

pub fn get_nested_types_from_type<'a>(ty: &'a Ty<'a>) -> Vec<&'a Ty<'a>> {

    let mut types: Vec<&'a Ty<'a>> = Vec::new();
    types.push(ty);

    match ty.kind {
        TyKind::Slice(sub_ty) | TyKind::Array(sub_ty, _)
        | TyKind::Rptr(_, rustc_hir::MutTy{ty: sub_ty, ..}) => {
            let mut sub_types : Vec<&'a Ty<'a>> = get_nested_types_from_type(sub_ty);
            types.append(&mut sub_types);
        },
        TyKind::Tup(tuple_args) => {
            for tuple_arg in tuple_args {
                let mut sub_types : Vec<&'a Ty<'a>> = get_nested_types_from_type(tuple_arg);
                types.append(&mut sub_types);
            }
        },
        _ => ()
    }

    let (_, args) = get_defid_args_from_kind(&ty.kind);

    for arg in args.iter() {
        if let rustc_hir::GenericArg::Type(sub_ty) = *arg {
            let mut sub_types: Vec<&'a Ty<'a>> = get_nested_types_from_type(sub_ty);
            types.append(&mut sub_types);
        }
    }

    types
}

pub fn get_bounds_from_generics<'a, 'tcx>(  generics: &'a rustc_hir::Generics<'tcx>,
                                            hir_map:  &'a Map<'tcx>
                                         )
                                -> HashMap<DefId, rustc_hir::GenericBounds<'tcx>> {

    let mut bound_map : HashMap<DefId, rustc_hir::GenericBounds> = HashMap::new();

    for predicate in generics.predicates {
        if let rustc_hir::WherePredicate::BoundPredicate (
                    rustc_hir::WhereBoundPredicate {
                        bounded_ty,
                        bounds,
                        ..
                    }
                ) = predicate {

            if let (Some(def_id), _) = get_defid_args_from_kind(&bounded_ty.kind) {
                bound_map.insert(def_id, bounds);
            }
        }
    }

    for param in generics.params {
        if let rustc_hir::GenericParamKind::Type{..} = param.kind {
            if !bound_map.contains_key(&param.def_id.to_def_id()) {
                bound_map.insert(param.def_id.to_def_id(), &[]);
            }
        }
    }
    bound_map
}

pub fn get_lifetime_lifetime_bounds<'a>(generics: &'a rustc_hir::Generics)
                                -> Vec<(LifetimeName, LifetimeName)> {

    let mut lifetime_bounds : Vec<(LifetimeName, LifetimeName)> = Vec::new();

    for predicate in generics.predicates {
        if let rustc_hir::WherePredicate::RegionPredicate (
                    rustc_hir::WhereRegionPredicate {
                        lifetime,
                        bounds,
                        ..
                    }
                ) = predicate {

            for bound in *bounds {
                if let rustc_hir::GenericBound::Outlives(sub_lifetime) = bound {
                    lifetime_bounds.push((lifetime.name, sub_lifetime.name));
                }
            }
        }
    }
    lifetime_bounds
}

pub fn compare_lifetimes(lifetime1: &rustc_hir::LifetimeName,
                         lifetime2: &rustc_hir::LifetimeName) -> bool {

    match (*lifetime1, *lifetime2) {

        (rustc_hir::LifetimeName::Param(def_id1, _),
            rustc_hir::LifetimeName::Param(def_id2, _)) => {
            if def_id1 == def_id2 {return true;}
            else {return false;}
        },

        _ => {
            if lifetime1 == lifetime2 {return true;}
            else {return false;}
        }
    }
}


pub fn is_user_defined_lifetime(lifetime: Option<&rustc_hir::LifetimeName>) -> bool {

    match lifetime {
        Some(rustc_hir::LifetimeName::Param(_, Plain(ident))) => {
            if (ident.as_str() == "'_") {return false;}
            else {return true;}
        },
        Some(rustc_hir::LifetimeName::Static) => {return true;},
        _ => {return false;}
    }
}

pub fn check_if_closure<'tcx>(bounds: &'tcx rustc_hir::GenericBounds<'tcx>) -> bool {
    for bound in *bounds {
        if let rustc_hir::GenericBound::LangItemTrait(lang_item, _, _, _) = bound {

            match lang_item {
                rustc_hir::lang_items::LangItem::Fn
                // | rustc_hir::lang_items::LangItem::FnOnce
                | rustc_hir::lang_items::LangItem::FnMut
                // | rustc_hir::lang_items::LangItem::FnOnceOutput
                    => {return true;},

                _ => ()
            }
        }
        if let rustc_hir::GenericBound::Trait(
                rustc_hir::PolyTraitRef{
                    trait_ref: rustc_hir::TraitRef{
                        path: rustc_hir::Path{
                            res: rustc_hir::def::Res::Def(_, def_id),
                        ..},
                    ..},
                ..}, _
            ) = bound {

            let def_str: String  = format!("{:?}", def_id);
            let def_name: String = def_str[..(def_str.len()-1)].split("::").last().unwrap().to_string();
            if def_name == "Fn" || def_name == "FnMut"
                // || def_name == "FnOnce" || def_name == "FnOnceOutput"
            {
                return true;
            }
        }
    }
    false
}

pub fn get_lifetime_from_type(inp_ty: &Ty) -> (Option<rustc_hir::LifetimeName>, Mutability) {

    let mut mutability = Mutability::Not;
    let mut lifetime: Option<rustc_hir::LifetimeName> = None;

    let sub_types: Vec<&Ty> = get_nested_types_from_type(&inp_ty);

    for ty in sub_types {

        if let TyKind::Rptr(rl, mut_ty) = &ty.kind {
            // if debug {
                // println!("{:?}, {:?}", rl, mut_ty);
            // }
            if lifetime == None {
                lifetime = Some(rl.name);
            }
            if mut_ty.mutbl == Mutability::Mut {
                mutability = mut_ty.mutbl;
                lifetime = Some(rl.name);
                break;
            }
        }
        else {
            let (_, args) = get_defid_args_from_kind(&ty.kind);

            for arg in args.iter() {
                if let rustc_hir::GenericArg::Lifetime(rustc_hir::Lifetime{name, ..}) = *arg {
                    if lifetime == None {
                        lifetime = Some(*name);
                    }
                    break;
                }
            }
        }
        // if let Some(_) = lifetime {
            // break;
        // }
    }
    (lifetime, mutability)
}

pub fn check_if_contains_lifetimes(hir_map: &Map) -> bool {

    for item_id in hir_map.items() {

        let item = hir_map.expect_item(item_id.owner_id.def_id);

        // First check all functions
        if let rustc_hir::ItemKind::Fn(fn_sig, generics, body_id) = &item.kind {

            // Check returned value
            if let rustc_hir::FnRetTy::Return(ret_type) = fn_sig.decl.output {
                let (ret_lifetime, _) = get_lifetime_from_type(&ret_type);
                if is_user_defined_lifetime(ret_lifetime.as_ref()) {return true;}
            }

            // Check input arguments
            for inp in fn_sig.decl.inputs.iter() {
                let (inp_lifetime, _) = get_lifetime_from_type(&inp);
                if is_user_defined_lifetime(inp_lifetime.as_ref()) {return true;}
            }

            // Check trait bounds
            let bounds_map = get_bounds_from_generics(&generics, &hir_map);
            for (&def_id, &bounds) in bounds_map.iter() {
                for bound in bounds.iter() {
                    if let rustc_hir::GenericBound::Outlives(lifetime) = *bound {
                        if is_user_defined_lifetime(Some(&lifetime.name)) {return true;}
                    }
                }
            }
            continue;
        }

        // Then check all functions within Implementations
        if let rustc_hir::ItemKind::Impl(this_impl) = &item.kind {

            for internal_item in this_impl.items {
                if let Some(rustc_hir::Node::ImplItem(
                                rustc_hir::ImplItem{
                                    kind: rustc_hir::ImplItemKind::Fn(fn_sig, body_id),
                                    generics,
                                    ..
                                }
                            )) = hir_map.find(internal_item.id.hir_id()) {

                    // Check returned value
                    if let rustc_hir::FnRetTy::Return(ret_type) = fn_sig.decl.output {
                        let (ret_lifetime, _) = get_lifetime_from_type(&ret_type);
                        if is_user_defined_lifetime(ret_lifetime.as_ref()) {return true;}
                    }

                    // Check input arguments
                    for inp in fn_sig.decl.inputs.iter() {
                        let (inp_lifetime, _) = get_lifetime_from_type(&inp);
                        if is_user_defined_lifetime(inp_lifetime.as_ref()) {return true;}
                    }

                    // Check trait bounds
                    let bounds_map = get_bounds_from_generics(&generics, &hir_map);
                    for (&def_id, &bounds) in bounds_map.iter() {
                        for bound in bounds.iter() {
                            if let rustc_hir::GenericBound::Outlives(lifetime) = *bound {
                                if is_user_defined_lifetime(Some(&lifetime.name)) {return true;}
                            }
                        }
                    }
                }
                continue;
            }
        }

        // Then check all structures
        if let rustc_hir::ItemKind::Struct(_, generics) = &item.kind {
            // Check trait bounds
            let bounds_map = get_bounds_from_generics(&generics, &hir_map);
            for (&def_id, &bounds) in bounds_map.iter() {
                for bound in bounds.iter() {
                    if let rustc_hir::GenericBound::Outlives(lifetime) = *bound {
                        if is_user_defined_lifetime(Some(&lifetime.name)) {return true;}
                    }
                }
            }
        }
    }
    false
}
