use gobject_ast::model::{
    FileModel, FunctionDeclItem, FunctionDefItem, SourceLocation, StructField, TopLevelItem,
    TypeDefItem, TypeInfo,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Fix, Rule, Violation},
};

pub struct QemuCoroutineFnPosition;

/// Annotations that belong before the return type. The `co_wrapper` family is
/// consumed by scripts/block-coroutine-wrapper.py, which accepts them in front
/// of the return type since "block-coroutine-wrapper: accept annotations before
/// return type".
const ANNOTATION_MODIFIERS: &[&str] = &[
    "coroutine_fn",
    "coroutine_mixed_fn",
    "no_coroutine_fn",
    "co_wrapper",
    "co_wrapper_mixed",
    "co_wrapper_bdrv_rdlock",
    "co_wrapper_mixed_bdrv_rdlock",
    "no_co_wrapper",
    "no_co_wrapper_bdrv_rdlock",
    "no_co_wrapper_bdrv_wrlock",
];

impl QemuCoroutineFnPosition {
    /// Deletion range for each misplaced annotation. Annotations separated by
    /// a single space share it, so the ranges never overlap: the run keeps its
    /// trailing space, or its leading one when it ends a line.
    fn delete_fixes(source: &[u8], misplaced: &[&SourceLocation]) -> Vec<Fix> {
        let mut fixes = Vec::with_capacity(misplaced.len());
        let mut run_start = 0;
        while run_start < misplaced.len() {
            let mut run_end = run_start + 1;
            while run_end < misplaced.len()
                && misplaced[run_end - 1].end_byte + 1 == misplaced[run_end].start_byte
                && source.get(misplaced[run_end].start_byte - 1) == Some(&b' ')
            {
                run_end += 1;
            }

            let run = &misplaced[run_start..run_end];
            let trailing_space = run
                .last()
                .is_some_and(|location| source.get(location.end_byte) == Some(&b' '));
            for (index, location) in run.iter().enumerate() {
                let mut del_start = location.start_byte;
                let mut del_end = location.end_byte;
                if let Some(next) = run.get(index + 1) {
                    del_end = next.start_byte;
                } else if trailing_space {
                    del_end += 1;
                }
                if index == 0 && !trailing_space && del_start > 0 && source[del_start - 1] == b' ' {
                    del_start -= 1;
                }
                fixes.push(Fix::delete(del_start, del_end));
            }

            run_start = run_end;
        }
        fixes
    }

    fn check_modifiers(
        &self,
        modifiers: &[SourceLocation],
        return_type: &TypeInfo,
        item_location: &SourceLocation,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        let source = item_location.source();
        let misplaced: Vec<&SourceLocation> = modifiers
            .iter()
            .filter(|location| {
                location.as_str().is_some_and(|modifier| {
                    ANNOTATION_MODIFIERS.contains(&modifier)
                        && location.start_byte >= return_type.location.end_byte
                })
            })
            .collect();

        // Reinsert every misplaced annotation in one go, after any annotation
        // that is already in place and before the return type, so storage
        // class specifiers keep their place. The annotations keep their
        // original order; the first violation carries the insertion.
        let insert_at = modifiers
            .iter()
            .filter(|location| location.start_byte < return_type.location.end_byte)
            .filter(|location| {
                location
                    .as_str()
                    .is_some_and(|modifier| ANNOTATION_MODIFIERS.contains(&modifier))
            })
            .map(|location| (location.end_byte + 1).min(source.len()))
            .max()
            .unwrap_or(return_type.location.start_byte);

        let mut moved = String::new();
        for location in &misplaced {
            if let Some(modifier) = location.as_str() {
                moved.push_str(modifier);
                moved.push(' ');
            }
        }

        let delete_fixes = Self::delete_fixes(source, &misplaced);
        for (index, location) in misplaced.iter().enumerate() {
            let Some(modifier) = location.as_str() else {
                continue;
            };
            let mut fixes = vec![delete_fixes[index].clone()];
            if index == 0 {
                fixes.push(Fix::new(insert_at, insert_at, moved.clone()));
            }
            violations.push(self.violation_with_fixes(
                &file.path,
                location.line,
                location.column,
                format!("`{modifier}` should be placed before the return type"),
                fixes,
            ));
        }
    }

    fn check_struct_fields(
        &self,
        fields: &[StructField],
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for field in fields {
            if let Some(callable) = &field.callable {
                self.check_modifiers(
                    &callable.macro_modifier_locations,
                    &field.field_type,
                    &field.location,
                    file,
                    violations,
                );
            }
            self.check_struct_fields(&field.inner_fields, file, violations);
        }
    }
}

impl Rule for QemuCoroutineFnPosition {
    fn name(&self) -> &'static str {
        "qemu:coroutine_fn_position"
    }

    fn description(&self) -> &'static str {
        "Ensure coroutine annotations appear before the return type, not between return type and function name"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(include_str!(
            "../../../docs/rules/qemu_coroutine_fn_position.md"
        ))
    }

    fn category(&self) -> Category {
        Category::Style
    }

    fn fixable(&self) -> bool {
        true
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn opt_in_reason(&self) -> Option<&'static str> {
        Some("QEMU-specific coroutine annotation position check")
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_modifiers(
            &func.macro_modifier_locations,
            &func.return_type,
            &func.location,
            file,
            violations,
        );
    }

    fn check_func_decl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDeclItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_modifiers(
            &func.macro_modifier_locations,
            &func.return_type,
            &func.location,
            file,
            violations,
        );
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        for (_path, file) in ast_context.iter_all_files() {
            for func in file.iter_function_definitions() {
                self.check_func_impl(ast_context, config, func, file, violations);
            }
            for func in file.iter_function_declarations() {
                self.check_func_decl(ast_context, config, func, file, violations);
            }
            for item in file.iter_all_items() {
                let fields = match item {
                    TopLevelItem::TypeDefinition(TypeDefItem::Struct { fields, .. }) => fields,
                    TopLevelItem::TypeDefinition(TypeDefItem::Typedef {
                        struct_fields, ..
                    }) => struct_fields,
                    _ => continue,
                };
                self.check_struct_fields(fields, file, violations);
            }
        }
    }
}
