//! Database: owns a [`Catalog`] and [`Storage`] for a process's
//! lifetime and runs the full pipeline (lex, parse, analyse, execute)
//! for SQL text. This is the crate's main entry point; everything
//! else is a stage this type wires together.

use crate::analyser::Analyser;
use crate::catalog::Catalog;
use crate::error::Result;
use crate::executor::{ExecResult, Executor};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::storage::Storage;

#[cfg(test)]
mod tests;

#[derive(Default)]
pub struct Database {
    catalog: Catalog,
    storage: Storage,
}

impl Database {
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs a single SQL statement end-to-end: lex, parse, analyse,
    /// execute. Returns `Ok(None)` if `sql` contains no statements
    /// (e.g. empty input, or only whitespace/comments). If `sql`
    /// contains more than one `;`-separated statement, only the first
    /// runs, see [`Database::execute_all`] to run a whole batch.
    pub fn execute(&mut self, sql: &str) -> Result<Option<ExecResult>> {
        Ok(self.execute_all(sql)?.into_iter().next())
    }

    /// Runs every `;`-separated statement in `sql`, in order,
    /// stopping at the first error. Statements already applied before
    /// that error are **not** rolled back, see the executor module
    /// doc comment on atomicity, which is per-statement, not
    /// per-batch.
    pub fn execute_all(&mut self, sql: &str) -> Result<Vec<ExecResult>> {
        let tokens = Lexer::new(sql).tokenise()?;
        let statements = Parser::new(tokens).parse()?;
        let mut results = Vec::with_capacity(statements.len());
        for statement in statements {
            let bound = Analyser::new(&self.catalog).analyse(statement)?;
            let result = Executor::new(&mut self.catalog, &mut self.storage).execute(bound)?;
            results.push(result);
        }
        Ok(results)
    }

    /// Read-only access to the current schema, e.g. for a future REPL
    /// to print `\d table_name` style output.
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
}
