use std::{borrow::Cow, mem};

use modula_utils::{hashbrown::HashSet, HashMap};
use wgpu::ShaderSource;

/// A shader module source, the start of a shader module should be lines with '//use mod_name' for dependencies.  
/// Lines can be included or excluded based on flags.  
/// A line containing '//if(condition)' where condition is either a flag name or '!condition', '(condition)&(condition)' or '(condition)|(condition)', where whitespace is not allowed will start a conditional section.  
/// This section should be ended by '//endif', conditional segments can be nested, and //else blocks can be added.  
pub struct ShaderModuleSource {
    source: String,
}

impl ShaderModuleSource {
    pub fn new(source: String) -> Self {
        Self { source }
    }
}

pub struct ShaderBundler {
    libraries: HashMap<String, ShaderLibrary>,
}

#[derive(Debug)]
pub enum ShaderBundlerError {
    ModuleAlreadyExists,
    UnknownDependency(String),
    InvalidCondition(String),
    CommentError(String),
}

impl ShaderBundler {
    pub fn new() -> Self {
        Self {
            libraries: HashMap::new(),
        }
    }

    /// Adds a library to the bundler, modules can add dependencies by adding lines containing //use lib_name in the start of the source
    /// Libraries are not supposed to add uniforms, however this is not checked by the bundler
    pub fn add_library(
        &mut self,
        name: String,
        source: ShaderModuleSource,
    ) -> Result<(), ShaderBundlerError> {
        let dependencies = get_dependencies(&source);
        match self.libraries.try_insert(
            name,
            ShaderLibrary {
                source,
                dependencies,
            },
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(ShaderBundlerError::ModuleAlreadyExists),
        }
    }

    /// Bundles a shader
    /// Interface is supposed to implement the vertex and and fragment (or compute main), while depending on functions that must be implemented by implementor
    /// No promises are checked while bundling
    pub fn bundle(
        &self,
        interface: &ShaderModuleSource,
        implementor: &ShaderModuleSource,
        flags: &[&str],
    ) -> Result<ShaderSource, ShaderBundlerError> {
        let mut res = String::new();
        let flags = flags.iter().map(|f| (*f).into()).collect();
        for dep in dependency_list(self, interface, implementor)? {
            let code: Vec<_> = self.libraries[&dep].source.source.split("\n").collect();
            let applied = apply_flags(&code, &flags, true)?.0.join("\n");
            res.push_str(&applied);
            res.push('\n');
        }
        Ok(ShaderSource::Wgsl(Cow::Owned(res)))
    }
}

struct ShaderLibrary {
    source: ShaderModuleSource,
    dependencies: Vec<String>,
}
enum ConditionToken {
    Parenthesie(bool),
    Operator(char),
    Literal(String),
}

fn dependency_list(
    bundler: &ShaderBundler,
    interface: &ShaderModuleSource,
    implementor: &ShaderModuleSource,
) -> Result<Vec<String>, ShaderBundlerError> {
    let mut queue = get_dependencies(interface);
    queue.append(&mut get_dependencies(implementor));
    let mut seen: HashSet<_> = queue.into_iter().collect();
    // to removes repeating elements
    let mut queue: Vec<_> = seen.clone().into_iter().collect();
    let mut res = Vec::new();
    while let Some(e) = queue.pop() {
        if seen.contains(&e) {
            continue;
        }
        for dep in &bundler
            .libraries
            .get(&e)
            .ok_or_else(|| ShaderBundlerError::UnknownDependency(e.clone()))?
            .dependencies
        {
            queue.push(dep.clone());
        }
        seen.insert(e.clone());
        res.push(e);
    }
    Ok(res)
}

fn apply_flags(
    code: &[&str],
    flags: &HashSet<String>,
    keep: bool,
) -> Result<(Vec<String>, usize), ShaderBundlerError> {
    let mut i = 0;
    let mut res = Vec::new();
    let mut in_else = false;

    while i < code.len() {
        let inst = &code[i];
        let trimmed = inst.trim();
        if trimmed == "//endif" {
            break;
        }
        if trimmed == "//else" {
            if in_else {
                return Err(ShaderBundlerError::CommentError(
                    "Found //else twice".into(),
                ));
            }
            in_else = true;
        }
        // calculating even if not keep, because it runs recursion to keep scopes
        // very stupid indeed...
        let mut sub = if is_if(trimmed) {
            let cond = &trimmed[5..trimmed.len() - 1];
            let res = eval_condition(cond, flags)
                .ok_or_else(|| ShaderBundlerError::InvalidCondition(cond.into()))?;
            let block;
            (block, i) = apply_flags(&code[i + 1..], flags, res)?;
            block
        } else {
            vec![(*inst).into()]
        };
        // same as !=, should only run if in_else or keep, not if both
        if keep ^ in_else {
            res.append(&mut sub);
        }
        i += 1;
    }
    todo!()
}

fn is_if(line: &str) -> bool {
    line.starts_with("//if(") && line.ends_with(")")
}

fn eval_condition(condition: &str, flags: &HashSet<String>) -> Option<bool> {
    eval_tokens(&tokenize(condition)?, flags)
}

fn eval_tokens(tokens: &[ConditionToken], flags: &HashSet<String>) -> Option<bool> {
    if tokens.is_empty() {
        return None;
    }
    match &tokens[0] {
        ConditionToken::Literal(lit) => (tokens.len() == 1).then(|| flags.contains(lit)),
        ConditionToken::Operator('!') => Some(!eval_tokens(&tokens[1..], flags)?),
        ConditionToken::Parenthesie(true) => {
            // insane pattern abuse
            if let ([_, first @ .., _], rest) = until_closing(tokens)? {
                if let [ConditionToken::Operator(op), ConditionToken::Parenthesie(true), last @ .., ConditionToken::Parenthesie(false)] =
                    rest
                {
                    let a = eval_tokens(first, flags)?;
                    let b = eval_tokens(last, flags)?;
                    match op {
                        '&' => Some(a && b),
                        '|' => Some(a || b),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn until_closing(tokens: &[ConditionToken]) -> Option<(&[ConditionToken], &[ConditionToken])> {
    let mut counter = 0;
    for (idx, token) in tokens.iter().enumerate() {
        match token {
            ConditionToken::Parenthesie(open) => {
                if *open {
                    counter += 1;
                } else {
                    counter -= 1;
                    if counter == 0 {
                        return Some((&tokens[1..idx], &tokens[idx + 1..]));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn tokenize(condition: &str) -> Option<Vec<ConditionToken>> {
    let mut cur = String::new();
    let mut res = Vec::new();
    for c in condition.chars() {
        let token = if c == '(' {
            Some(ConditionToken::Parenthesie(true))
        } else if c == ')' {
            Some(ConditionToken::Parenthesie(false))
        } else if "&|!".contains(c) {
            Some(ConditionToken::Operator(c))
        } else if !c.is_alphanumeric() && c != '_' {
            // invalid character
            return None;
        } else {
            None
        };
        if let Some(token) = token {
            if !cur.is_empty() {
                res.push(ConditionToken::Literal(mem::take(&mut cur)));
            }
            res.push(token);
        }
    }
    if !cur.is_empty() {
        res.push(ConditionToken::Literal(cur));
    }
    Some(res)
}

fn get_dependencies(module: &ShaderModuleSource) -> Vec<String> {
    let mut dependencies = Vec::new();
    for ln in module.source.split("\n") {
        if ln.len() >= 6 && &ln[..6] == "//use " {
            dependencies.push(ln[..6].trim().to_string());
        } else {
            break;
        }
    }
    dependencies
}
