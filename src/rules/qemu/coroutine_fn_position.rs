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

const COROUTINE_MODIFIERS: &[&str] = &["coroutine_fn", "coroutine_mixed_fn", "no_coroutine_fn"];

fn find_word_in_region(source: &[u8], start: usize, end: usize, word: &[u8]) -> Option<usize> {
    let region = &source[start..end];
    region
        .windows(word.len())
        .position(|w| {
            if w != word {
                return false;
            }
            let abs = start + (w.as_ptr() as usize - region.as_ptr() as usize);
            let before_ok =
                abs == 0 || !source[abs - 1].is_ascii_alphanumeric() && source[abs - 1] != b'_';
            let after_ok = abs + word.len() >= source.len()
                || !source[abs + word.len()].is_ascii_alphanumeric()
                    && source[abs + word.len()] != b'_';
            before_ok && after_ok
        })
        .map(|offset| start + offset)
}

fn byte_to_line_col(source: &[u8], byte_offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for &b in &source[..byte_offset] {
        if b == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

impl QemuCoroutineFnPosition {
    fn check_modifiers(
        &self,
        modifiers: &[String],
        return_type: &TypeInfo,
        item_location: &SourceLocation,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for modifier in modifiers {
            if !COROUTINE_MODIFIERS.contains(&modifier.as_str()) {
                continue;
            }

            let source = item_location.source();
            let search_start = return_type.location.end_byte;
            let search_end = item_location.end_byte;

            let Some(mod_start) =
                find_word_in_region(source, search_start, search_end, modifier.as_bytes())
            else {
                continue;
            };
            let mod_end = mod_start + modifier.len();

            let mut del_start = mod_start;
            let mut del_end = mod_end;

            if del_end < source.len() && source[del_end] == b' ' {
                del_end += 1;
            } else if del_start > 0 && source[del_start - 1] == b' ' {
                del_start -= 1;
            }

            let delete_fix = Fix::delete(del_start, del_end);
            let insert_fix = Fix::new(
                item_location.start_byte,
                item_location.start_byte,
                format!("{modifier} "),
            );

            let (line, col) = byte_to_line_col(source, mod_start);
            violations.push(self.violation_with_fixes(
                &file.path,
                line,
                col,
                format!("`{modifier}` should be placed before the return type"),
                vec![delete_fix, insert_fix],
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
                    &callable.macro_modifiers,
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
            &func.macro_modifiers,
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
            &func.macro_modifiers,
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
