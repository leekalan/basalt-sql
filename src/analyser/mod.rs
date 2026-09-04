//! Analyser: binds a parsed [`Statement`] against a [`Catalog`],
//! resolving column names to indices/types and rejecting statements
//! that are syntactically valid but semantically meaningless (unknown
//! tables/columns, type mismatches). Purely a checking + resolving
//! stage, no row data is touched here; see [`crate::executor`] for
//! that.

pub mod bound;
pub mod error;

pub use bound::*;
pub use error::{AnalyseError, Result};

use std::collections::HashSet;

use crate::catalog::{Catalog, TableSchema};
use crate::parser::{
    BinaryOp, CreateTableStatement, DeleteStatement, Expr, InsertStatement, SelectColumns,
    SelectStatement, Statement, UpdateStatement,
};
use crate::types::DataType;

#[cfg(test)]
mod tests;

/// Binds statements against a [`Catalog`]. Borrows the catalog rather
/// than owning it: binding is read-only, and row storage / schema
/// mutation both happen later, in the executor.
pub struct Analyser<'a> {
    catalog: &'a Catalog,
}

impl<'a> Analyser<'a> {
    pub fn new(catalog: &'a Catalog) -> Self {
        Self { catalog }
    }

    /// Binds a single statement, or fails with the first semantic
    /// error found.
    pub fn analyse(&self, statement: Statement) -> Result<BoundStatement> {
        match statement {
            Statement::Select(s) => self.analyse_select(s).map(BoundStatement::Select),
            Statement::Insert(s) => self.analyse_insert(s).map(BoundStatement::Insert),
            Statement::Update(s) => self.analyse_update(s).map(BoundStatement::Update),
            Statement::Delete(s) => self.analyse_delete(s).map(BoundStatement::Delete),
            Statement::CreateTable(s) => self
                .analyse_create_table(s)
                .map(BoundStatement::CreateTable),
        }
    }

    fn schema_for(&self, table: &str) -> Result<&'a TableSchema> {
        self.catalog
            .schema(table)
            .ok_or_else(|| AnalyseError::UnknownTable {
                name: table.to_string(),
            })
    }

    fn resolve_column(&self, schema: &TableSchema, table: &str, name: &str) -> Result<BoundColumn> {
        schema
            .columns
            .iter()
            .enumerate()
            .find(|(_, c)| c.name == name)
            .map(|(index, c)| BoundColumn {
                name: c.name.clone(),
                index,
                ty: c.ty.clone(),
            })
            .ok_or_else(|| AnalyseError::UnknownColumn {
                table: table.to_string(),
                column: name.to_string(),
            })
    }

    fn analyse_select(&self, stmt: SelectStatement) -> Result<BoundSelect> {
        let schema = self.schema_for(&stmt.table)?;
        let columns = match stmt.columns {
            SelectColumns::All => schema
                .columns
                .iter()
                .enumerate()
                .map(|(index, c)| BoundColumn {
                    name: c.name.clone(),
                    index,
                    ty: c.ty.clone(),
                })
                .collect(),
            SelectColumns::List(names) => names
                .into_iter()
                .map(|name| self.resolve_column(schema, &stmt.table, &name))
                .collect::<Result<Vec<_>>>()?,
        };
        let filter = stmt
            .filter
            .map(|e| self.bind_expr(&e, schema, &stmt.table, true))
            .transpose()?;
        Ok(BoundSelect {
            table: stmt.table,
            columns,
            filter,
        })
    }

    fn analyse_insert(&self, stmt: InsertStatement) -> Result<BoundInsert> {
        let schema = self.schema_for(&stmt.table)?;
        if stmt.values.len() != schema.columns.len() {
            return Err(AnalyseError::ValueCountMismatch {
                table: stmt.table.clone(),
                expected: schema.columns.len(),
                found: stmt.values.len(),
            });
        }
        // `false`: a VALUES clause has no source row to resolve a
        // column reference against. Only constant expressions are
        // allowed. See AnalyseError::ColumnInValues.
        let values = stmt
            .values
            .iter()
            .map(|e| self.bind_expr(e, schema, &stmt.table, false))
            .collect::<Result<Vec<_>>>()?;

        for (bound, col) in values.iter().zip(&schema.columns) {
            if let Some(found) = bound.static_type()
                && found != col.ty
            {
                return Err(AnalyseError::TypeMismatch {
                    expected: format!("{:?} for column '{}'", col.ty, col.name),
                    found: format!("{found:?}"),
                });
            }
        }

        Ok(BoundInsert {
            table: stmt.table,
            values,
        })
    }

    fn analyse_update(&self, stmt: UpdateStatement) -> Result<BoundUpdate> {
        let schema = self.schema_for(&stmt.table)?;
        let assignments = stmt
            .assignments
            .into_iter()
            .map(|(name, expr)| -> Result<(BoundColumn, BoundExpr)> {
                let column = self.resolve_column(schema, &stmt.table, &name)?;
                // `true`: unlike VALUES, SET legitimately references
                // other columns of the same row (`SET balance =
                // balance * 2`), evaluated by the executor against
                // the row's pre update values.
                let bound = self.bind_expr(&expr, schema, &stmt.table, true)?;
                if let Some(found) = bound.static_type()
                    && found != column.ty
                {
                    return Err(AnalyseError::TypeMismatch {
                        expected: format!("{:?} for column '{}'", column.ty, column.name),
                        found: format!("{found:?}"),
                    });
                }
                Ok((column, bound))
            })
            .collect::<Result<Vec<_>>>()?;
        let filter = stmt
            .filter
            .map(|e| self.bind_expr(&e, schema, &stmt.table, true))
            .transpose()?;
        Ok(BoundUpdate {
            table: stmt.table,
            assignments,
            filter,
        })
    }

    fn analyse_delete(&self, stmt: DeleteStatement) -> Result<BoundDelete> {
        let schema = self.schema_for(&stmt.table)?;
        let filter = stmt
            .filter
            .map(|e| self.bind_expr(&e, schema, &stmt.table, true))
            .transpose()?;
        Ok(BoundDelete {
            table: stmt.table,
            filter,
        })
    }

    /// `CREATE TABLE` has no existing schema to bind against.
    /// Checked instead for: the table not already
    /// existing ([`Catalog::register_table`] would silently overwrite
    /// it otherwise), and no duplicate column names within the
    /// declaration itself.
    fn analyse_create_table(&self, stmt: CreateTableStatement) -> Result<CreateTableStatement> {
        if self.catalog.has_table(&stmt.table) {
            return Err(AnalyseError::TableAlreadyExists {
                name: stmt.table.clone(),
            });
        }
        let mut seen = HashSet::new();
        for col in &stmt.columns {
            if !seen.insert(col.name.clone()) {
                return Err(AnalyseError::DuplicateColumn {
                    name: col.name.clone(),
                });
            }
        }
        Ok(stmt)
    }

    /// Recursively binds an [`Expr`] tree, resolving column names and
    /// rejecting statically-detectable type errors along the way.
    ///
    /// `allow_columns` is `false` only for `INSERT ... VALUES`, the
    /// one context with no row to resolve a column reference against.
    fn bind_expr(
        &self,
        expr: &Expr,
        schema: &TableSchema,
        table: &str,
        allow_columns: bool,
    ) -> Result<BoundExpr> {
        match expr {
            Expr::Column(name) => {
                if !allow_columns {
                    return Err(AnalyseError::ColumnInValues { name: name.clone() });
                }
                let bound = self.resolve_column(schema, table, name)?;
                Ok(BoundExpr::Column {
                    name: bound.name,
                    index: bound.index,
                    ty: bound.ty,
                })
            }
            Expr::Literal(value) => Ok(BoundExpr::Literal(value.clone())),
            Expr::Not(inner) => {
                let inner = self.bind_expr(inner, schema, table, allow_columns)?;
                match inner.static_type() {
                    Some(DataType::Boolean) | None => {}
                    Some(other) => {
                        return Err(AnalyseError::TypeMismatch {
                            expected: "BOOLEAN".into(),
                            found: format!("{other:?}"),
                        });
                    }
                }
                Ok(BoundExpr::Not(Box::new(inner)))
            }
            Expr::Neg(inner) => {
                let inner = self.bind_expr(inner, schema, table, allow_columns)?;
                match inner.static_type() {
                    Some(DataType::Integer) | Some(DataType::Float) | None => {}
                    Some(other) => {
                        return Err(AnalyseError::TypeMismatch {
                            expected: "a numeric type".into(),
                            found: format!("{other:?}"),
                        });
                    }
                }
                Ok(BoundExpr::Neg(Box::new(inner)))
            }
            Expr::BinaryOp { left, op, right } => {
                let left = self.bind_expr(left, schema, table, allow_columns)?;
                let right = self.bind_expr(right, schema, table, allow_columns)?;
                check_binary_op_types(*op, &left, &right)?;
                Ok(BoundExpr::BinaryOp {
                    left: Box::new(left),
                    op: *op,
                    right: Box::new(right),
                })
            }
            Expr::IsNull { expr, negated } => {
                // No type restriction. IS NULL is valid on a column
                // or expression of any type.
                let inner = self.bind_expr(expr, schema, table, allow_columns)?;
                Ok(BoundExpr::IsNull {
                    expr: Box::new(inner),
                    negated: *negated,
                })
            }
        }
    }
}

/// Static legality check for a [`BinaryOp`] given its (already bound)
/// operand types. `NULL` operands (`static_type() == None`) skip
/// checking entirely. See [`BoundExpr::static_type`].
fn check_binary_op_types(op: BinaryOp, left: &BoundExpr, right: &BoundExpr) -> Result<()> {
    let (Some(lt), Some(rt)) = (left.static_type(), right.static_type()) else {
        return Ok(());
    };
    let numeric = |t: &DataType| matches!(t, DataType::Integer | DataType::Float);

    match op {
        BinaryOp::And | BinaryOp::Or => {
            if lt != DataType::Boolean || rt != DataType::Boolean {
                return Err(AnalyseError::TypeMismatch {
                    expected: "BOOLEAN on both sides".into(),
                    found: format!("{lt:?} and {rt:?}"),
                });
            }
        }
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
            if !numeric(&lt) || !numeric(&rt) {
                return Err(AnalyseError::TypeMismatch {
                    expected: "numeric operands".into(),
                    found: format!("{lt:?} and {rt:?}"),
                });
            }
        }
        BinaryOp::Eq | BinaryOp::NotEq => {
            let comparable = lt == rt || (numeric(&lt) && numeric(&rt));
            if !comparable {
                return Err(AnalyseError::TypeMismatch {
                    expected: format!("a type comparable to {lt:?}"),
                    found: format!("{rt:?}"),
                });
            }
        }
        BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
            let orderable =
                (numeric(&lt) && numeric(&rt)) || (lt == DataType::Text && rt == DataType::Text);
            if !orderable {
                return Err(AnalyseError::TypeMismatch {
                    expected: "two numeric or two TEXT operands".into(),
                    found: format!("{lt:?} and {rt:?}"),
                });
            }
        }
    }
    Ok(())
}
