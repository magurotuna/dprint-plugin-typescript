use std::cmp::Ordering;

pub fn cmp_module_specifiers(a: &str, b: &str, cmp_text: impl Fn(&str, &str) -> Ordering) -> Ordering {
    let a_info = get_module_specifier_info(a);
    let b_info = get_module_specifier_info(b);

    return match a_info.kind {
        ModuleSpecifierKind::Absolute => {
            match b_info.kind {
                ModuleSpecifierKind::Absolute => compare_folder_items(&a_info, &b_info, cmp_text),
                ModuleSpecifierKind::Relative(_) => Ordering::Less,
            }
        },
        ModuleSpecifierKind::Relative(a_relative_count) => {
            match b_info.kind {
                ModuleSpecifierKind::Absolute => Ordering::Greater,
                ModuleSpecifierKind::Relative(b_relative_count) => {
                    match a_relative_count.cmp(&b_relative_count) {
                        Ordering::Greater => Ordering::Less,
                        Ordering::Less => Ordering::Greater,
                        Ordering::Equal => compare_folder_items(&a_info, &b_info, cmp_text),
                    }
                },
            }
        }
    };

    fn compare_folder_items(a_info: &ModuleSpecifierInfo, b_info: &ModuleSpecifierInfo, cmp_text: impl Fn(&str, &str) -> Ordering) -> Ordering {
        let max_len = std::cmp::min(a_info.folder_items.len(), b_info.folder_items.len());
        // compare the common items except for the potential file name at the end
        for i in 0..max_len - 1 {
            let a_text = a_info.folder_items[i];
            let b_text = b_info.folder_items[i];
            let ordering = cmp_text(a_text, b_text);
            if ordering != Ordering::Equal {
                return ordering;
            }
        }

        if a_info.folder_items.len() != b_info.folder_items.len() {
            // the import that has a folder name should appear before the one with a file name
            if a_info.folder_items.len() > b_info.folder_items.len() {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        } else {
            // compare the file names
            cmp_text(a_info.folder_items.last().unwrap(), b_info.folder_items.last().unwrap())
        }
    }
}

struct ModuleSpecifierInfo<'a> {
    kind: ModuleSpecifierKind,
    folder_items: Vec<&'a str>,
}

#[derive(Debug, PartialEq)]
enum ModuleSpecifierKind {
    Absolute,
    Relative(usize),
}

fn get_module_specifier_info<'a>(text: &'a str) -> ModuleSpecifierInfo<'a> {
    let no_quotes_text = &text[1..text.len() - 1];
    let parts = no_quotes_text.split('/').collect::<Vec<_>>();
    if parts[0] == "." || parts[0] == ".." {
        let mut relative_count = 0;
        let mut start_index = 0;
        let parts_len = parts.len();
        for (i, part) in parts.into_iter().enumerate() {
            if part == ".." {
                relative_count += 1;
            } else if part != "." {
                break;
            }

            // use 0 for last part since it does not have a slash after it
            let next_slash_width = if i == parts_len - 1 { 0 } else { 1 };
            start_index += part.len() + next_slash_width;
        }

        ModuleSpecifierInfo {
            folder_items: no_quotes_text[start_index..].split('/').collect(),
            kind: ModuleSpecifierKind::Relative(relative_count),
        }
    } else {
        ModuleSpecifierInfo {
            folder_items: no_quotes_text.split('/').collect(),
            kind: ModuleSpecifierKind::Absolute,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_should_compare_module_specifiers() {
        assert_eq!(cmp_module_specifiers("''", "'test'", |a, b| a.cmp(b)), Ordering::Less);
        assert_eq!(cmp_module_specifiers("'a'", "'b'", |a, b| a.cmp(b)), Ordering::Less);
        assert_eq!(cmp_module_specifiers("'b'", "'a'", |a, b| a.cmp(b)), Ordering::Greater);
        assert_eq!(cmp_module_specifiers("'a'", "'a'", |a, b| a.cmp(b)), Ordering::Equal);
        assert_eq!(cmp_module_specifiers("'a'", "'./a'", |a, b| a.cmp(b)), Ordering::Less);
        assert_eq!(cmp_module_specifiers("'./a'", "'a'", |a, b| a.cmp(b)), Ordering::Greater);
        assert_eq!(cmp_module_specifiers("'./a'", "'./a'", |a, b| a.cmp(b)), Ordering::Equal);
        assert_eq!(cmp_module_specifiers("'../a'", "'./a'", |a, b| a.cmp(b)), Ordering::Less);
        assert_eq!(cmp_module_specifiers("'./a'", "'../a'", |a, b| a.cmp(b)), Ordering::Greater);
        assert_eq!(cmp_module_specifiers("'../../a'", "'../a'", |a, b| a.cmp(b)), Ordering::Less);
        assert_eq!(cmp_module_specifiers("'../a'", "'../../a'", |a, b| a.cmp(b)), Ordering::Greater);
        assert_eq!(cmp_module_specifiers("'..'", "'test'", |a, b| a.cmp(b)), Ordering::Greater);
    }

    #[test]
    fn it_should_get_module_specifier_info_when_empty() {
        let result = get_module_specifier_info("''");
        assert_eq!(result.folder_items, vec![""]);
        assert_eq!(result.kind, ModuleSpecifierKind::Absolute);
    }

    #[test]
    fn it_should_get_module_specifier_info_for_absolute() {
        // double quotes
        let result = get_module_specifier_info(r#""testing/asdf""#);
        assert_eq!(result.folder_items, vec!["testing", "asdf"]);
        assert_eq!(result.kind, ModuleSpecifierKind::Absolute);

        // single quotes
        let result = get_module_specifier_info("'testing'");
        assert_eq!(result.folder_items, vec!["testing"]);
        assert_eq!(result.kind, ModuleSpecifierKind::Absolute);
    }

    #[test]
    fn it_should_get_module_specifier_info_for_relative() {
        let result = get_module_specifier_info("'./a'");
        assert_eq!(result.folder_items, vec!["a"]);
        assert_eq!(result.kind, ModuleSpecifierKind::Relative(0));

        let result = get_module_specifier_info("'./../testing-test/t'");
        assert_eq!(result.folder_items, vec!["testing-test", "t"]);
        assert_eq!(result.kind, ModuleSpecifierKind::Relative(1));

        let result = get_module_specifier_info("'../asdf'");
        assert_eq!(result.folder_items, vec!["asdf"]);
        assert_eq!(result.kind, ModuleSpecifierKind::Relative(1));

        let result = get_module_specifier_info("'../../../test'");
        assert_eq!(result.folder_items, vec!["test"]);
        assert_eq!(result.kind, ModuleSpecifierKind::Relative(3));
    }
}
