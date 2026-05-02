use super::*;

impl Formatter {
    pub(in crate::formatter) fn format_block(&mut self, block: &Block) {
        if block.statements.is_empty() {
            self.out.push_str("{}");
            return;
        }

        self.out.push_str("{\n");
        self.indent += 1;
        for statement in &block.statements {
            self.write_indent();
            self.format_statement(statement);
            self.out.push('\n');
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    pub(in crate::formatter) fn format_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                initializer,
            } => {
                let binding = if *mutable { "let mut" } else { "let" };
                let _ = write!(
                    self.out,
                    "{binding} {name}: {} = {};",
                    format_type_ref(ty),
                    format_expr(initializer)
                );
            }
            StmtKind::Assign { target, value } => {
                let _ = write!(
                    self.out,
                    "{} = {};",
                    format_expr(target),
                    format_expr(value)
                );
            }
            StmtKind::Expr { expr } => {
                let _ = write!(self.out, "{};", format_expr(expr));
            }
            StmtKind::Return { value } => {
                if let Some(expr) = value {
                    let _ = write!(self.out, "return {};", format_expr(expr));
                } else {
                    self.out.push_str("return;");
                }
            }
            StmtKind::Break => self.out.push_str("break;"),
            StmtKind::Continue => self.out.push_str("continue;"),
            StmtKind::Match { scrutinee, arms } => self.format_match_statement(scrutinee, arms),
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.format_if_statement(condition, then_branch, else_branch.as_ref()),
            StmtKind::While { condition, body } => {
                let _ = write!(self.out, "while ({}) ", format_expr(condition));
                self.format_block(body);
            }
            StmtKind::For {
                initializer,
                condition,
                step,
                body,
            } => {
                self.out.push_str("for (");
                if let Some(statement) = initializer {
                    self.out
                        .push_str(&format_for_header_statement(statement.as_ref()));
                }
                self.out.push(';');
                if let Some(expr) = condition {
                    self.out.push(' ');
                    self.out.push_str(&format_expr(expr));
                }
                self.out.push(';');
                if let Some(statement) = step {
                    self.out.push(' ');
                    self.out
                        .push_str(&format_for_header_statement(statement.as_ref()));
                }
                self.out.push_str(") ");
                self.format_block(body);
            }
            StmtKind::ForIn {
                binding,
                iterable,
                body,
            } => {
                let binding_prefix = if binding.mutable { "let mut" } else { "let" };
                let _ = write!(
                    self.out,
                    "for ({} {}: {} in {}) ",
                    binding_prefix,
                    binding.name,
                    format_type_ref(&binding.ty),
                    format_expr(iterable)
                );
                self.format_block(body);
            }
            StmtKind::Block { block } => self.format_block(block),
        }
    }

    pub(in crate::formatter) fn format_if_statement(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&Block>,
    ) {
        let _ = write!(self.out, "if ({}) ", format_expr(condition));
        self.format_block(then_branch);
        if let Some(block) = else_branch {
            if let Some(else_if) = else_if_statement(block) {
                self.out.push_str(" else ");
                if let StmtKind::If {
                    condition,
                    then_branch,
                    else_branch,
                } = &else_if.kind
                {
                    self.format_if_statement(condition, then_branch, else_branch.as_ref());
                }
            } else {
                self.out.push_str(" else ");
                self.format_block(block);
            }
        }
    }

    pub(in crate::formatter) fn format_match_statement(
        &mut self,
        scrutinee: &Expr,
        arms: &[MatchArm],
    ) {
        let _ = write!(self.out, "match ({}) ", format_expr(scrutinee));
        if arms.is_empty() {
            self.out.push_str("{}");
            return;
        }

        self.out.push_str("{\n");
        self.indent += 1;
        for arm in arms {
            self.write_indent();
            let _ = write!(
                self.out,
                "{}{} => ",
                format_match_pattern(&arm.pattern),
                format_match_guard(arm.guard.as_ref())
            );
            self.format_block(&arm.body);
            self.out.push('\n');
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    pub(in crate::formatter) fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
    }
}
