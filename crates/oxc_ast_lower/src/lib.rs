#![allow(clippy::unused_self)]

use oxc_allocator::{Allocator, Box, Vec};
use oxc_ast::ast;
use oxc_hir::{hir, hir_builder::HirBuilder};
use oxc_span::GetSpan;

pub struct AstLower<'a> {
    hir: HirBuilder<'a>,
}

impl<'a> AstLower<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        Self { hir: HirBuilder::new(allocator) }
    }

    #[must_use]
    pub fn build(mut self, program: &ast::Program<'a>) -> hir::Program<'a> {
        self.lower_program(program)
    }

    #[must_use]
    pub fn lower_vec<T, R, F>(&mut self, items: &Vec<'a, T>, cb: F) -> Vec<'a, R>
    where
        F: Fn(&mut Self, &T) -> R,
    {
        let mut vec = self.hir.new_vec_with_capacity(items.len());
        for item in items {
            vec.push(cb(self, item));
        }
        vec
    }

    #[must_use]
    pub fn lower_statements(
        &mut self,
        stmts: &Vec<'a, ast::Statement<'a>>,
    ) -> Vec<'a, hir::Statement<'a>> {
        let mut vec = self.hir.new_vec_with_capacity(stmts.len());
        for stmt in stmts {
            if let Some(stmt) = self.lower_statement(stmt) {
                vec.push(stmt);
            }
        }
        vec
    }

    fn lower_program(&mut self, program: &ast::Program<'a>) -> hir::Program<'a> {
        let directives = self.lower_vec(&program.directives, Self::lower_directive);
        let statements = self.lower_statements(&program.body);
        self.hir.program(program.span, directives, statements)
    }

    fn lower_directive(&mut self, directive: &ast::Directive<'a>) -> hir::Directive<'a> {
        let expression = self.lower_string_literal(&directive.expression);
        self.hir.directive(directive.span, expression, directive.directive)
    }

    fn lower_statement_or_empty(&mut self, statement: &ast::Statement<'a>) -> hir::Statement<'a> {
        self.lower_statement(statement)
            .unwrap_or_else(|| self.hir.empty_statement(statement.span()))
    }

    fn lower_statement(&mut self, statement: &ast::Statement<'a>) -> Option<hir::Statement<'a>> {
        match statement {
            ast::Statement::BlockStatement(stmt) => Some(self.lower_block_statement(stmt)),
            ast::Statement::BreakStatement(stmt) => Some(self.lower_break_statement(stmt)),
            ast::Statement::ContinueStatement(stmt) => Some(self.lower_continue_statement(stmt)),
            ast::Statement::DebuggerStatement(stmt) => Some(self.lower_debugger_statement(stmt)),
            ast::Statement::DoWhileStatement(stmt) => Some(self.lower_do_while_statement(stmt)),
            ast::Statement::EmptyStatement(stmt) => Some(self.lower_empty_statement(stmt)),
            ast::Statement::ExpressionStatement(stmt) => {
                Some(self.lower_expression_statement(stmt))
            }
            ast::Statement::ForInStatement(stmt) => Some(self.lower_for_in_statement(stmt)),
            ast::Statement::ForOfStatement(stmt) => Some(self.lower_for_of_statement(stmt)),
            ast::Statement::ForStatement(stmt) => Some(self.lower_for_statement(stmt)),
            ast::Statement::IfStatement(stmt) => Some(self.lower_if_statement(stmt)),
            ast::Statement::LabeledStatement(stmt) => Some(self.lower_labeled_statement(stmt)),
            ast::Statement::ReturnStatement(stmt) => Some(self.lower_return_statement(stmt)),
            ast::Statement::SwitchStatement(stmt) => Some(self.lower_switch_statement(stmt)),
            ast::Statement::ThrowStatement(stmt) => Some(self.lower_throw_statement(stmt)),
            ast::Statement::TryStatement(stmt) => Some(self.lower_try_statement(stmt)),
            ast::Statement::WhileStatement(stmt) => Some(self.lower_while_statement(stmt)),
            ast::Statement::WithStatement(stmt) => Some(self.lower_with_statement(stmt)),
            ast::Statement::ModuleDeclaration(decl) => self.lower_module_declaration(decl),
            ast::Statement::Declaration(decl) => {
                self.lower_declaration(decl).map(hir::Statement::Declaration)
            }
        }
    }

    fn lower_block(&mut self, stmt: &ast::BlockStatement<'a>) -> Box<'a, hir::BlockStatement<'a>> {
        let body = self.lower_statements(&stmt.body);
        self.hir.block(stmt.span, body)
    }

    fn lower_block_statement(&mut self, stmt: &ast::BlockStatement<'a>) -> hir::Statement<'a> {
        let body = self.lower_statements(&stmt.body);
        self.hir.block_statement(stmt.span, body)
    }

    fn lower_break_statement(&mut self, stmt: &ast::BreakStatement) -> hir::Statement<'a> {
        let label = stmt.label.as_ref().map(|ident| self.lower_label_identifier(ident));
        self.hir.break_statement(stmt.span, label)
    }

    fn lower_continue_statement(&mut self, stmt: &ast::ContinueStatement) -> hir::Statement<'a> {
        let label = stmt.label.as_ref().map(|ident| self.lower_label_identifier(ident));
        self.hir.continue_statement(stmt.span, label)
    }

    fn lower_debugger_statement(&mut self, stmt: &ast::DebuggerStatement) -> hir::Statement<'a> {
        self.hir.debugger_statement(stmt.span)
    }

    fn lower_do_while_statement(&mut self, stmt: &ast::DoWhileStatement<'a>) -> hir::Statement<'a> {
        let body = self.lower_statement_or_empty(&stmt.body);
        let test = self.lower_expression(&stmt.test);
        self.hir.do_while_statement(stmt.span, body, test)
    }

    fn lower_empty_statement(&mut self, stmt: &ast::EmptyStatement) -> hir::Statement<'a> {
        self.hir.empty_statement(stmt.span)
    }

    fn lower_expression_statement(
        &mut self,
        stmt: &ast::ExpressionStatement<'a>,
    ) -> hir::Statement<'a> {
        let expression = self.lower_expression(&stmt.expression);
        self.hir.expression_statement(stmt.span, expression)
    }

    fn lower_for_statement(&mut self, stmt: &ast::ForStatement<'a>) -> hir::Statement<'a> {
        let init = stmt.init.as_ref().map(|init| self.lower_for_statement_init(init));
        let test = stmt.test.as_ref().map(|expr| self.lower_expression(expr));
        let update = stmt.update.as_ref().map(|expr| self.lower_expression(expr));
        let body = self.lower_statement_or_empty(&stmt.body);
        self.hir.for_statement(stmt.span, init, test, update, body)
    }

    fn lower_for_statement_init(
        &mut self,
        init: &ast::ForStatementInit<'a>,
    ) -> hir::ForStatementInit<'a> {
        match init {
            ast::ForStatementInit::VariableDeclaration(decl) => {
                hir::ForStatementInit::VariableDeclaration(self.lower_variable_declaration(decl))
            }
            ast::ForStatementInit::Expression(expr) => {
                hir::ForStatementInit::Expression(self.lower_expression(expr))
            }
        }
    }

    fn lower_for_in_statement(&mut self, stmt: &ast::ForInStatement<'a>) -> hir::Statement<'a> {
        let left = self.lower_for_statement_left(&stmt.left);
        let right = self.lower_expression(&stmt.right);
        let body = self.lower_statement_or_empty(&stmt.body);
        self.hir.for_in_statement(stmt.span, left, right, body)
    }

    fn lower_for_of_statement(&mut self, stmt: &ast::ForOfStatement<'a>) -> hir::Statement<'a> {
        let left = self.lower_for_statement_left(&stmt.left);
        let right = self.lower_expression(&stmt.right);
        let body = self.lower_statement_or_empty(&stmt.body);
        self.hir.for_of_statement(stmt.span, stmt.r#await, left, right, body)
    }

    fn lower_for_statement_left(
        &mut self,
        left: &ast::ForStatementLeft<'a>,
    ) -> hir::ForStatementLeft<'a> {
        match left {
            ast::ForStatementLeft::VariableDeclaration(decl) => {
                hir::ForStatementLeft::VariableDeclaration(self.lower_variable_declaration(decl))
            }
            ast::ForStatementLeft::AssignmentTarget(target) => {
                hir::ForStatementLeft::AssignmentTarget(self.lower_assignment_target(target))
            }
        }
    }

    fn lower_if_statement(&mut self, stmt: &ast::IfStatement<'a>) -> hir::Statement<'a> {
        let test = self.lower_expression(&stmt.test);
        let consequent = self.lower_statement_or_empty(&stmt.consequent);
        let alternate = stmt.alternate.as_ref().and_then(|stmt| self.lower_statement(stmt));
        self.hir.if_statement(stmt.span, test, consequent, alternate)
    }

    fn lower_labeled_statement(&mut self, stmt: &ast::LabeledStatement<'a>) -> hir::Statement<'a> {
        let label = self.lower_label_identifier(&stmt.label);
        let body = self.lower_statement_or_empty(&stmt.body);
        self.hir.labeled_statement(stmt.span, label, body)
    }

    fn lower_return_statement(&mut self, stmt: &ast::ReturnStatement<'a>) -> hir::Statement<'a> {
        let argument = stmt.argument.as_ref().map(|expr| self.lower_expression(expr));
        self.hir.return_statement(stmt.span, argument)
    }

    fn lower_switch_statement(&mut self, stmt: &ast::SwitchStatement<'a>) -> hir::Statement<'a> {
        let discriminant = self.lower_expression(&stmt.discriminant);
        let cases = self.lower_vec(&stmt.cases, Self::lower_switch_case);
        self.hir.switch_statement(stmt.span, discriminant, cases)
    }

    fn lower_switch_case(&mut self, case: &ast::SwitchCase<'a>) -> hir::SwitchCase<'a> {
        let test = case.test.as_ref().map(|expr| self.lower_expression(expr));
        let consequent = self.lower_statements(&case.consequent);
        self.hir.switch_case(case.span, test, consequent)
    }

    fn lower_throw_statement(&mut self, stmt: &ast::ThrowStatement<'a>) -> hir::Statement<'a> {
        let argument = self.lower_expression(&stmt.argument);
        self.hir.throw_statement(stmt.span, argument)
    }

    fn lower_try_statement(&mut self, stmt: &ast::TryStatement<'a>) -> hir::Statement<'a> {
        let block = self.lower_block(&stmt.block);
        let handler = stmt.handler.as_ref().map(|clause| self.lower_catch_clause(clause));
        let finalizer = stmt.finalizer.as_ref().map(|stmt| self.lower_block(stmt));
        self.hir.try_statement(stmt.span, block, handler, finalizer)
    }

    fn lower_catch_clause(
        &mut self,
        clause: &ast::CatchClause<'a>,
    ) -> Box<'a, hir::CatchClause<'a>> {
        let body = self.lower_block(&clause.body);
        let param = clause.param.as_ref().map(|pat| self.lower_binding_pattern(pat));
        self.hir.catch_clause(clause.span, param, body)
    }

    fn lower_while_statement(&mut self, stmt: &ast::WhileStatement<'a>) -> hir::Statement<'a> {
        let test = self.lower_expression(&stmt.test);
        let body = self.lower_statement_or_empty(&stmt.body);
        self.hir.while_statement(stmt.span, test, body)
    }

    fn lower_with_statement(&mut self, stmt: &ast::WithStatement<'a>) -> hir::Statement<'a> {
        let object = self.lower_expression(&stmt.object);
        let body = self.lower_statement_or_empty(&stmt.body);
        self.hir.with_statement(stmt.span, object, body)
    }

    fn lower_expression(&mut self, expr: &ast::Expression<'a>) -> hir::Expression<'a> {
        match expr {
            ast::Expression::BigintLiteral(lit) => {
                let lit = self.lower_bigint_literal(lit);
                self.hir.literal_bigint_expression(lit)
            }
            ast::Expression::BooleanLiteral(lit) => {
                let lit = self.lower_boolean_literal(lit);
                self.hir.literal_boolean_expression(lit)
            }
            ast::Expression::NullLiteral(lit) => {
                let lit = self.lower_null_literal(lit);
                self.hir.literal_null_expression(lit)
            }
            ast::Expression::NumberLiteral(lit) => {
                let lit = self.lower_number_literal(lit);
                self.hir.literal_number_expression(lit)
            }
            ast::Expression::RegExpLiteral(lit) => {
                let lit = self.lower_reg_expr_literal(lit);
                self.hir.literal_regexp_expression(lit)
            }
            ast::Expression::StringLiteral(lit) => {
                let lit = self.lower_string_literal(lit);
                self.hir.literal_string_expression(lit)
            }
            ast::Expression::TemplateLiteral(lit) => {
                let lit = self.lower_template_literal(lit);
                self.hir.literal_template_expression(lit)
            }
            ast::Expression::Identifier(ident) => {
                let lit = self.lower_identifier_reference(ident);
                self.hir.identifier_reference_expression(lit)
            }
            ast::Expression::MetaProperty(meta) => self.lower_meta_property(meta),
            ast::Expression::ArrayExpression(expr) => self.lower_array_expression(expr),
            ast::Expression::ArrowFunctionExpression(expr) => self.lower_arrow_expression(expr),
            ast::Expression::AssignmentExpression(expr) => self.lower_assignment_expression(expr),
            ast::Expression::AwaitExpression(expr) => self.lower_await_expression(expr),
            ast::Expression::BinaryExpression(expr) => self.lower_binary_expression(expr),
            ast::Expression::CallExpression(expr) => self.lower_call_expression(expr),
            ast::Expression::ChainExpression(expr) => self.lower_chain_expression(expr),
            ast::Expression::ClassExpression(expr) => self.lower_class_expression(expr),
            ast::Expression::ConditionalExpression(expr) => self.lower_conditional_expression(expr),
            ast::Expression::FunctionExpression(expr) => self.lower_function_expression(expr),
            ast::Expression::ImportExpression(expr) => self.lower_import_expression(expr),
            ast::Expression::LogicalExpression(expr) => self.lower_logical_expression(expr),
            ast::Expression::MemberExpression(expr) => self.lower_member_expression(expr),
            ast::Expression::NewExpression(expr) => self.lower_new_expression(expr),
            ast::Expression::ObjectExpression(expr) => self.lower_object_expression(expr),
            ast::Expression::PrivateInExpression(expr) => self.lower_private_in_expression(expr),
            ast::Expression::SequenceExpression(expr) => self.lower_sequence_expression(expr),
            ast::Expression::TaggedTemplateExpression(expr) => {
                self.lower_tagged_template_expression(expr)
            }
            ast::Expression::ThisExpression(expr) => self.lower_this_expression(expr),
            ast::Expression::UnaryExpression(expr) => self.lower_unary_expression(expr),
            ast::Expression::UpdateExpression(expr) => self.lower_update_expression(expr),
            ast::Expression::YieldExpression(expr) => self.lower_yield_expression(expr),
            ast::Expression::Super(expr) => self.lower_super(expr),
            ast::Expression::JSXElement(elem) => {
                // TODO: implement JSX
                let ident = self.hir.identifier_reference(elem.span, "void".into());
                self.hir.identifier_reference_expression(ident)
            }
            ast::Expression::JSXFragment(elem) => {
                // TODO: implement JSX
                let ident = self.hir.identifier_reference(elem.span, "void".into());
                self.hir.identifier_reference_expression(ident)
            }

            // Syntax trimmed for the following expressions
            ast::Expression::ParenthesizedExpression(expr) => {
                self.lower_expression(&expr.expression)
            }
            ast::Expression::TSAsExpression(expr) => self.lower_expression(&expr.expression),
            ast::Expression::TSSatisfiesExpression(expr) => self.lower_expression(&expr.expression),
            ast::Expression::TSNonNullExpression(expr) => self.lower_expression(&expr.expression),
            ast::Expression::TSTypeAssertion(expr) => self.lower_expression(&expr.expression),
            ast::Expression::TSInstantiationExpression(expr) => {
                self.lower_expression(&expr.expression)
            }
        }
    }

    fn lower_meta_property(&mut self, prop: &ast::MetaProperty) -> hir::Expression<'a> {
        let meta = self.lower_identifier_name(&prop.meta);
        let property = self.lower_identifier_name(&prop.property);
        self.hir.meta_property(prop.span, meta, property)
    }

    fn lower_array_expression(&mut self, expr: &ast::ArrayExpression<'a>) -> hir::Expression<'a> {
        let elements = self.lower_vec(&expr.elements, Self::lower_array_expression_element);
        self.hir.array_expression(expr.span, elements, expr.trailing_comma)
    }

    fn lower_array_expression_element(
        &mut self,
        elem: &ast::ArrayExpressionElement<'a>,
    ) -> hir::ArrayExpressionElement<'a> {
        match elem {
            ast::ArrayExpressionElement::SpreadElement(elem) => {
                let elem = self.lower_spread_element(elem);
                hir::ArrayExpressionElement::SpreadElement(elem)
            }
            ast::ArrayExpressionElement::Expression(expr) => {
                let expr = self.lower_expression(expr);
                hir::ArrayExpressionElement::Expression(expr)
            }
            ast::ArrayExpressionElement::Elision(span) => {
                hir::ArrayExpressionElement::Elision(*span)
            }
        }
    }

    fn lower_argument(&mut self, arg: &ast::Argument<'a>) -> hir::Argument<'a> {
        match arg {
            ast::Argument::SpreadElement(elem) => {
                let spread_element = self.lower_spread_element(elem);
                hir::Argument::SpreadElement(spread_element)
            }
            ast::Argument::Expression(expr) => {
                hir::Argument::Expression(self.lower_expression(expr))
            }
        }
    }

    fn lower_spread_element(
        &mut self,
        elem: &ast::SpreadElement<'a>,
    ) -> Box<'a, hir::SpreadElement<'a>> {
        let argument = self.lower_expression(&elem.argument);
        self.hir.spread_element(elem.span, argument)
    }

    fn lower_assignment_expression(
        &mut self,
        expr: &ast::AssignmentExpression<'a>,
    ) -> hir::Expression<'a> {
        let operator = match expr.operator {
            ast::AssignmentOperator::Assign => hir::AssignmentOperator::Assign,
            ast::AssignmentOperator::Addition => hir::AssignmentOperator::Addition,
            ast::AssignmentOperator::Subtraction => hir::AssignmentOperator::Subtraction,
            ast::AssignmentOperator::Multiplication => hir::AssignmentOperator::Multiplication,
            ast::AssignmentOperator::Division => hir::AssignmentOperator::Division,
            ast::AssignmentOperator::Remainder => hir::AssignmentOperator::Remainder,
            ast::AssignmentOperator::ShiftLeft => hir::AssignmentOperator::ShiftLeft,
            ast::AssignmentOperator::ShiftRight => hir::AssignmentOperator::ShiftRight,
            ast::AssignmentOperator::ShiftRightZeroFill => {
                hir::AssignmentOperator::ShiftRightZeroFill
            }
            ast::AssignmentOperator::BitwiseOR => hir::AssignmentOperator::BitwiseOR,
            ast::AssignmentOperator::BitwiseXOR => hir::AssignmentOperator::BitwiseXOR,
            ast::AssignmentOperator::BitwiseAnd => hir::AssignmentOperator::BitwiseAnd,
            ast::AssignmentOperator::LogicalAnd => hir::AssignmentOperator::LogicalAnd,
            ast::AssignmentOperator::LogicalOr => hir::AssignmentOperator::LogicalOr,
            ast::AssignmentOperator::LogicalNullish => hir::AssignmentOperator::LogicalNullish,
            ast::AssignmentOperator::Exponential => hir::AssignmentOperator::Exponential,
        };
        let left = self.lower_assignment_target(&expr.left);
        let right = self.lower_expression(&expr.right);
        self.hir.assignment_expression(expr.span, operator, left, right)
    }

    fn lower_arrow_expression(&mut self, expr: &ast::ArrowExpression<'a>) -> hir::Expression<'a> {
        let params = self.lower_formal_parameters(&expr.params);
        let body = self.lower_function_body(&expr.body);
        self.hir.arrow_expression(
            expr.span,
            expr.expression,
            expr.generator,
            expr.r#async,
            params,
            body,
        )
    }

    fn lower_await_expression(&mut self, expr: &ast::AwaitExpression<'a>) -> hir::Expression<'a> {
        let argument = self.lower_expression(&expr.argument);
        self.hir.await_expression(expr.span, argument)
    }

    fn lower_binary_expression(&mut self, expr: &ast::BinaryExpression<'a>) -> hir::Expression<'a> {
        let left = self.lower_expression(&expr.left);
        let operator = match expr.operator {
            ast::BinaryOperator::Equality => hir::BinaryOperator::Equality,
            ast::BinaryOperator::Inequality => hir::BinaryOperator::Inequality,
            ast::BinaryOperator::StrictEquality => hir::BinaryOperator::StrictEquality,
            ast::BinaryOperator::StrictInequality => hir::BinaryOperator::StrictInequality,
            ast::BinaryOperator::LessThan => hir::BinaryOperator::LessThan,
            ast::BinaryOperator::LessEqualThan => hir::BinaryOperator::LessEqualThan,
            ast::BinaryOperator::GreaterThan => hir::BinaryOperator::GreaterThan,
            ast::BinaryOperator::GreaterEqualThan => hir::BinaryOperator::GreaterEqualThan,
            ast::BinaryOperator::ShiftLeft => hir::BinaryOperator::ShiftLeft,
            ast::BinaryOperator::ShiftRight => hir::BinaryOperator::ShiftRight,
            ast::BinaryOperator::ShiftRightZeroFill => hir::BinaryOperator::ShiftRightZeroFill,
            ast::BinaryOperator::Addition => hir::BinaryOperator::Addition,
            ast::BinaryOperator::Subtraction => hir::BinaryOperator::Subtraction,
            ast::BinaryOperator::Multiplication => hir::BinaryOperator::Multiplication,
            ast::BinaryOperator::Division => hir::BinaryOperator::Division,
            ast::BinaryOperator::Remainder => hir::BinaryOperator::Remainder,
            ast::BinaryOperator::BitwiseOR => hir::BinaryOperator::BitwiseOR,
            ast::BinaryOperator::BitwiseXOR => hir::BinaryOperator::BitwiseXOR,
            ast::BinaryOperator::BitwiseAnd => hir::BinaryOperator::BitwiseAnd,
            ast::BinaryOperator::In => hir::BinaryOperator::In,
            ast::BinaryOperator::Instanceof => hir::BinaryOperator::Instanceof,
            ast::BinaryOperator::Exponential => hir::BinaryOperator::Exponential,
        };
        let right = self.lower_expression(&expr.right);
        self.hir.binary_expression(expr.span, left, operator, right)
    }

    fn lower_call_expression(&mut self, expr: &ast::CallExpression<'a>) -> hir::Expression<'a> {
        let callee = self.lower_expression(&expr.callee);
        let arguments = self.lower_vec(&expr.arguments, Self::lower_argument);
        self.hir.call_expression(expr.span, callee, arguments, expr.optional)
    }

    fn lower_chain_expression(&mut self, expr: &ast::ChainExpression<'a>) -> hir::Expression<'a> {
        let expression = match &expr.expression {
            ast::ChainElement::CallExpression(call_expr) => {
                let hir::Expression::CallExpression(call_expr) = self.lower_call_expression(call_expr) else {
                    unreachable!()
                };
                hir::ChainElement::CallExpression(call_expr)
            }
            ast::ChainElement::MemberExpression(member_expr) => {
                let hir::Expression::MemberExpression(member_expr) = self.lower_member_expression(member_expr) else {
                    unreachable!()
                };
                hir::ChainElement::MemberExpression(member_expr)
            }
        };
        self.hir.chain_expression(expr.span, expression)
    }

    fn lower_class_expression(&mut self, class: &ast::Class<'a>) -> hir::Expression<'a> {
        let class = self.lower_class(class);
        self.hir.class_expression(class)
    }

    fn lower_conditional_expression(
        &mut self,
        expr: &ast::ConditionalExpression<'a>,
    ) -> hir::Expression<'a> {
        let test = self.lower_expression(&expr.test);
        let consequent = self.lower_expression(&expr.consequent);
        let alternate = self.lower_expression(&expr.alternate);
        self.hir.conditional_expression(expr.span, test, consequent, alternate)
    }

    fn lower_function_expression(&mut self, func: &ast::Function<'a>) -> hir::Expression<'a> {
        let func = self.lower_function(func);
        self.hir.function_expression(func)
    }

    fn lower_import_expression(&mut self, expr: &ast::ImportExpression<'a>) -> hir::Expression<'a> {
        let source = self.lower_expression(&expr.source);
        let arguments = self.lower_vec(&expr.arguments, Self::lower_expression);
        self.hir.import_expression(expr.span, source, arguments)
    }

    fn lower_logical_expression(
        &mut self,
        expr: &ast::LogicalExpression<'a>,
    ) -> hir::Expression<'a> {
        let left = self.lower_expression(&expr.left);
        let operator = match expr.operator {
            ast::LogicalOperator::Or => hir::LogicalOperator::Or,
            ast::LogicalOperator::And => hir::LogicalOperator::And,
            ast::LogicalOperator::Coalesce => hir::LogicalOperator::Coalesce,
        };
        let right = self.lower_expression(&expr.right);
        self.hir.logical_expression(expr.span, left, operator, right)
    }

    fn lower_member_expr(&mut self, expr: &ast::MemberExpression<'a>) -> hir::MemberExpression<'a> {
        match expr {
            ast::MemberExpression::ComputedMemberExpression(expr) => {
                self.lower_computed_member_expression(expr)
            }
            ast::MemberExpression::StaticMemberExpression(expr) => {
                self.lower_static_member_expression(expr)
            }
            ast::MemberExpression::PrivateFieldExpression(expr) => {
                self.lower_private_field_expression(expr)
            }
        }
    }

    fn lower_member_expression(&mut self, expr: &ast::MemberExpression<'a>) -> hir::Expression<'a> {
        let member_expr = self.lower_member_expr(expr);
        self.hir.member_expression(member_expr)
    }

    fn lower_computed_member_expression(
        &mut self,
        expr: &ast::ComputedMemberExpression<'a>,
    ) -> hir::MemberExpression<'a> {
        let object = self.lower_expression(&expr.object);
        let expression = self.lower_expression(&expr.expression);
        self.hir.computed_member_expression(expr.span, object, expression, expr.optional)
    }

    fn lower_static_member_expression(
        &mut self,
        expr: &ast::StaticMemberExpression<'a>,
    ) -> hir::MemberExpression<'a> {
        let object = self.lower_expression(&expr.object);
        let property = self.lower_identifier_name(&expr.property);
        self.hir.static_member_expression(expr.span, object, property, expr.optional)
    }

    fn lower_private_field_expression(
        &mut self,
        expr: &ast::PrivateFieldExpression<'a>,
    ) -> hir::MemberExpression<'a> {
        let object = self.lower_expression(&expr.object);
        let field = self.lower_private_identifier(&expr.field);
        self.hir.private_field_expression(expr.span, object, field, expr.optional)
    }

    fn lower_new_expression(&mut self, expr: &ast::NewExpression<'a>) -> hir::Expression<'a> {
        let callee = self.lower_expression(&expr.callee);
        let arguments = self.lower_vec(&expr.arguments, Self::lower_argument);
        self.hir.new_expression(expr.span, callee, arguments)
    }

    fn lower_object_expression(&mut self, expr: &ast::ObjectExpression<'a>) -> hir::Expression<'a> {
        let properties = self.lower_vec(&expr.properties, Self::lower_object_property);
        self.hir.object_expression(expr.span, properties, expr.trailing_comma)
    }

    fn lower_object_property(&mut self, prop: &ast::ObjectProperty<'a>) -> hir::ObjectProperty<'a> {
        match prop {
            ast::ObjectProperty::Property(property) => {
                let property = self.lower_property(property);
                hir::ObjectProperty::Property(property)
            }
            ast::ObjectProperty::SpreadProperty(spread_element) => {
                let spread_element = self.lower_spread_element(spread_element);
                hir::ObjectProperty::SpreadProperty(spread_element)
            }
        }
    }

    fn lower_property(&mut self, prop: &ast::Property<'a>) -> Box<'a, hir::Property<'a>> {
        let kind = match prop.kind {
            ast::PropertyKind::Init => hir::PropertyKind::Init,
            ast::PropertyKind::Get => hir::PropertyKind::Get,
            ast::PropertyKind::Set => hir::PropertyKind::Set,
        };
        let key = self.lower_property_key(&prop.key);
        let value = self.lower_property_value(&prop.value);
        self.hir.property(prop.span, kind, key, value, prop.method, prop.shorthand, prop.computed)
    }

    fn lower_property_key(&mut self, key: &ast::PropertyKey<'a>) -> hir::PropertyKey<'a> {
        match key {
            ast::PropertyKey::Identifier(ident) => {
                let ident = self.lower_identifier_name(ident);
                self.hir.property_key_identifier(ident)
            }
            ast::PropertyKey::PrivateIdentifier(ident) => {
                let ident = self.lower_private_identifier(ident);
                self.hir.property_key_private_identifier(ident)
            }
            ast::PropertyKey::Expression(expr) => {
                hir::PropertyKey::Expression(self.lower_expression(expr))
            }
        }
    }

    fn lower_property_value(&mut self, value: &ast::PropertyValue<'a>) -> hir::PropertyValue<'a> {
        match value {
            ast::PropertyValue::Pattern(pat) => {
                hir::PropertyValue::Pattern(self.lower_binding_pattern(pat))
            }
            ast::PropertyValue::Expression(expr) => {
                hir::PropertyValue::Expression(self.lower_expression(expr))
            }
        }
    }

    fn lower_private_in_expression(
        &mut self,
        expr: &ast::PrivateInExpression<'a>,
    ) -> hir::Expression<'a> {
        let left = self.lower_private_identifier(&expr.left);
        let right = self.lower_expression(&expr.right);
        self.hir.private_in_expression(expr.span, left, right)
    }

    fn lower_sequence_expression(
        &mut self,
        expr: &ast::SequenceExpression<'a>,
    ) -> hir::Expression<'a> {
        let expressions = self.lower_vec(&expr.expressions, Self::lower_expression);
        self.hir.sequence_expression(expr.span, expressions)
    }

    fn lower_tagged_template_expression(
        &mut self,
        expr: &ast::TaggedTemplateExpression<'a>,
    ) -> hir::Expression<'a> {
        let tag = self.lower_expression(&expr.tag);
        let quasi = self.lower_template_literal(&expr.quasi);
        self.hir.tagged_template_expression(expr.span, tag, quasi)
    }

    fn lower_this_expression(&mut self, expr: &ast::ThisExpression) -> hir::Expression<'a> {
        self.hir.this_expression(expr.span)
    }

    fn lower_unary_expression(&mut self, expr: &ast::UnaryExpression<'a>) -> hir::Expression<'a> {
        let operator = match expr.operator {
            ast::UnaryOperator::UnaryNegation => hir::UnaryOperator::UnaryNegation,
            ast::UnaryOperator::UnaryPlus => hir::UnaryOperator::UnaryPlus,
            ast::UnaryOperator::LogicalNot => hir::UnaryOperator::LogicalNot,
            ast::UnaryOperator::BitwiseNot => hir::UnaryOperator::BitwiseNot,
            ast::UnaryOperator::Typeof => hir::UnaryOperator::Typeof,
            ast::UnaryOperator::Void => hir::UnaryOperator::Void,
            ast::UnaryOperator::Delete => hir::UnaryOperator::Delete,
        };
        let argument = self.lower_expression(&expr.argument);
        self.hir.unary_expression(expr.span, operator, expr.prefix, argument)
    }

    fn lower_update_expression(&mut self, expr: &ast::UpdateExpression<'a>) -> hir::Expression<'a> {
        let operator = match expr.operator {
            ast::UpdateOperator::Increment => hir::UpdateOperator::Increment,
            ast::UpdateOperator::Decrement => hir::UpdateOperator::Decrement,
        };
        let argument = self.lower_simple_assignment_target(&expr.argument);
        self.hir.update_expression(expr.span, operator, expr.prefix, argument)
    }

    fn lower_yield_expression(&mut self, expr: &ast::YieldExpression<'a>) -> hir::Expression<'a> {
        let argument = expr.argument.as_ref().map(|expr| self.lower_expression(expr));
        self.hir.yield_expression(expr.span, expr.delegate, argument)
    }

    fn lower_super(&mut self, expr: &ast::Super) -> hir::Expression<'a> {
        self.hir.super_expression(expr.span)
    }

    fn lower_assignment_target(
        &mut self,
        target: &ast::AssignmentTarget<'a>,
    ) -> hir::AssignmentTarget<'a> {
        match target {
            ast::AssignmentTarget::SimpleAssignmentTarget(target) => {
                hir::AssignmentTarget::SimpleAssignmentTarget(
                    self.lower_simple_assignment_target(target),
                )
            }
            ast::AssignmentTarget::AssignmentTargetPattern(target) => {
                hir::AssignmentTarget::AssignmentTargetPattern(
                    self.lower_assignment_target_pattern(target),
                )
            }
        }
    }

    fn lower_simple_assignment_target(
        &mut self,
        target: &ast::SimpleAssignmentTarget<'a>,
    ) -> hir::SimpleAssignmentTarget<'a> {
        match target {
            ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => {
                let ident = self.lower_identifier_reference(ident);
                self.hir.assignment_target_identifier(ident)
            }
            ast::SimpleAssignmentTarget::MemberAssignmentTarget(member_expr) => {
                let member_expr = self.lower_member_expr(member_expr);
                self.hir.member_assignment_target(member_expr)
            }
            ast::SimpleAssignmentTarget::TSAsExpression(expr) => {
                self.lower_assignment_target_expression(&expr.expression)
            }
            ast::SimpleAssignmentTarget::TSSatisfiesExpression(expr) => {
                self.lower_assignment_target_expression(&expr.expression)
            }
            ast::SimpleAssignmentTarget::TSNonNullExpression(expr) => {
                self.lower_assignment_target_expression(&expr.expression)
            }
            ast::SimpleAssignmentTarget::TSTypeAssertion(expr) => {
                self.lower_assignment_target_expression(&expr.expression)
            }
        }
    }

    fn lower_assignment_target_expression(
        &mut self,
        expr: &ast::Expression<'a>,
    ) -> hir::SimpleAssignmentTarget<'a> {
        match expr {
            ast::Expression::Identifier(ident) => {
                let ident = self.lower_identifier_reference(ident);
                self.hir.assignment_target_identifier(ident)
            }
            ast::Expression::MemberExpression(member_expr) => {
                let member_expr = self.lower_member_expr(member_expr);
                self.hir.member_assignment_target(member_expr)
            }
            _ => unreachable!(),
        }
    }

    fn lower_assignment_target_pattern(
        &mut self,
        pat: &ast::AssignmentTargetPattern<'a>,
    ) -> hir::AssignmentTargetPattern<'a> {
        match pat {
            ast::AssignmentTargetPattern::ArrayAssignmentTarget(target) => {
                let target = self.lower_array_assignment_target(target);
                hir::AssignmentTargetPattern::ArrayAssignmentTarget(target)
            }
            ast::AssignmentTargetPattern::ObjectAssignmentTarget(target) => {
                let target = self.lower_object_assignment_target(target);
                hir::AssignmentTargetPattern::ObjectAssignmentTarget(target)
            }
        }
    }

    fn lower_array_assignment_target(
        &mut self,
        target: &ast::ArrayAssignmentTarget<'a>,
    ) -> Box<'a, hir::ArrayAssignmentTarget<'a>> {
        let mut elements = self.hir.new_vec_with_capacity(target.elements.len());
        for elem in &target.elements {
            let elem = elem.as_ref().map(|elem| self.lower_assignment_target_maybe_default(elem));
            elements.push(elem);
        }
        let rest = target.rest.as_ref().map(|target| self.lower_assignment_target(target));
        self.hir.array_assignment_target(target.span, elements, rest, target.trailing_comma)
    }

    fn lower_object_assignment_target(
        &mut self,
        target: &ast::ObjectAssignmentTarget<'a>,
    ) -> Box<'a, hir::ObjectAssignmentTarget<'a>> {
        let properties = self.lower_vec(&target.properties, Self::lower_assignment_target_property);
        let rest = target.rest.as_ref().map(|target| self.lower_assignment_target(target));
        self.hir.object_assignment_target(target.span, properties, rest)
    }

    fn lower_assignment_target_maybe_default(
        &mut self,
        target: &ast::AssignmentTargetMaybeDefault<'a>,
    ) -> hir::AssignmentTargetMaybeDefault<'a> {
        match target {
            ast::AssignmentTargetMaybeDefault::AssignmentTarget(target) => {
                let target = self.lower_assignment_target(target);
                hir::AssignmentTargetMaybeDefault::AssignmentTarget(target)
            }
            ast::AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(target) => {
                let target = self.lower_assignment_target_with_default(target);
                hir::AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(target)
            }
        }
    }

    fn lower_assignment_target_with_default(
        &mut self,
        target: &ast::AssignmentTargetWithDefault<'a>,
    ) -> Box<'a, hir::AssignmentTargetWithDefault<'a>> {
        let binding = self.lower_assignment_target(&target.binding);
        let init = self.lower_expression(&target.init);
        self.hir.assignment_target_with_default(target.span, binding, init)
    }

    fn lower_assignment_target_property(
        &mut self,
        property: &ast::AssignmentTargetProperty<'a>,
    ) -> hir::AssignmentTargetProperty<'a> {
        match property {
            ast::AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(ident) => {
                let ident = self.lower_assignment_target_property_identifier(ident);
                hir::AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(ident)
            }
            ast::AssignmentTargetProperty::AssignmentTargetPropertyProperty(prop) => {
                let prop = self.lower_assignment_target_property_property(prop);
                hir::AssignmentTargetProperty::AssignmentTargetPropertyProperty(prop)
            }
        }
    }

    fn lower_assignment_target_property_identifier(
        &mut self,
        ident: &ast::AssignmentTargetPropertyIdentifier<'a>,
    ) -> Box<'a, hir::AssignmentTargetPropertyIdentifier<'a>> {
        let binding = self.lower_identifier_reference(&ident.binding);
        let init = ident.init.as_ref().map(|expr| self.lower_expression(expr));
        self.hir.assignment_target_property_identifier(ident.span, binding, init)
    }

    fn lower_assignment_target_property_property(
        &mut self,
        property: &ast::AssignmentTargetPropertyProperty<'a>,
    ) -> Box<'a, hir::AssignmentTargetPropertyProperty<'a>> {
        let name = self.lower_property_key(&property.name);
        let binding = self.lower_assignment_target_maybe_default(&property.binding);
        self.hir.assignment_target_property_property(property.span, name, binding)
    }

    // fn lower_jsx_element(&mut self, elem: &ast::JSXElement<'a>) {
    // todo!()
    // }

    // fn lower_jsx_opening_element(&mut self, elem: &ast::JSXOpeningElement<'a>) {
    // todo!()
    // }

    // fn lower_jsx_element_name(&mut self, __name: &ast::JSXElementName<'a>) {
    // todo!()
    // }

    // fn lower_jsx_attribute_item(&mut self, item: &ast::JSXAttributeItem<'a>) {
    // todo!()
    // }

    // fn lower_jsx_attribute(&mut self, attribute: &ast::JSXAttribute<'a>) {
    // todo!()
    // }

    // fn lower_jsx_spread_attribute(&mut self, attribute: &ast::JSXSpreadAttribute<'a>) {
    // todo!()
    // }

    // fn lower_jsx_attribute_value(&mut self, value: &ast::JSXAttributeValue<'a>) {
    // todo!()
    // }

    // fn lower_jsx_expression_container(&mut self, expr: &ast::JSXExpressionContainer<'a>) {
    // todo!()
    // }

    // fn lower_jsx_expression(&mut self, expr: &ast::JSXExpression<'a>) {
    // todo!()
    // }

    // fn lower_jsx_fragment(&mut self, elem: &ast::JSXFragment<'a>) {
    // todo!()
    // }

    // fn lower_jsx_child(&mut self, child: &ast::JSXChild<'a>) {
    // todo!()
    // }

    // fn lower_jsx_spread_child(&mut self, child: &ast::JSXSpreadChild<'a>) {
    // todo!()
    // }

    /* ----------  Pattern ---------- */

    fn lower_binding_pattern(&mut self, pat: &ast::BindingPattern<'a>) -> hir::BindingPattern<'a> {
        match &pat.kind {
            ast::BindingPatternKind::BindingIdentifier(ident) => {
                let ident = self.lower_binding_identifier(ident);
                self.hir.binding_identifier_pattern(ident)
            }
            ast::BindingPatternKind::ObjectPattern(pat) => self.lower_object_pattern(pat),
            ast::BindingPatternKind::ArrayPattern(pat) => self.lower_array_pattern(pat),
            ast::BindingPatternKind::RestElement(elem) => {
                let rest_element = self.lower_rest_element(elem);
                hir::BindingPattern::RestElement(rest_element)
            }
            ast::BindingPatternKind::AssignmentPattern(pat) => self.lower_assignment_pattern(pat),
        }
    }

    fn lower_object_pattern(&mut self, pat: &ast::ObjectPattern<'a>) -> hir::BindingPattern<'a> {
        let properties = self.lower_vec(&pat.properties, Self::lower_object_pattern_property);
        self.hir.object_pattern(pat.span, properties)
    }

    fn lower_object_pattern_property(
        &mut self,
        prop: &ast::ObjectPatternProperty<'a>,
    ) -> hir::ObjectPatternProperty<'a> {
        match prop {
            ast::ObjectPatternProperty::Property(prop) => {
                hir::ObjectPatternProperty::Property(self.lower_property(prop))
            }
            ast::ObjectPatternProperty::RestElement(elem) => {
                hir::ObjectPatternProperty::RestElement(self.lower_rest_element(elem))
            }
        }
    }

    fn lower_array_pattern(&mut self, pat: &ast::ArrayPattern<'a>) -> hir::BindingPattern<'a> {
        let mut elements = self.hir.new_vec_with_capacity(pat.elements.len());
        for elem in &pat.elements {
            let elem = elem.as_ref().map(|pat| self.lower_binding_pattern(pat));
            elements.push(elem);
        }
        self.hir.array_pattern(pat.span, elements)
    }

    fn lower_rest_element(&mut self, pat: &ast::RestElement<'a>) -> Box<'a, hir::RestElement<'a>> {
        let argument = self.lower_binding_pattern(&pat.argument);
        self.hir.rest_element(pat.span, argument)
    }

    fn lower_assignment_pattern(
        &mut self,
        pat: &ast::AssignmentPattern<'a>,
    ) -> hir::BindingPattern<'a> {
        let left = self.lower_binding_pattern(&pat.left);
        let right = self.lower_expression(&pat.right);
        self.hir.assignment_pattern(pat.span, left, right)
    }

    /* ----------  Identifier ---------- */

    fn lower_identifier_reference(
        &mut self,
        ident: &ast::IdentifierReference,
    ) -> hir::IdentifierReference {
        self.hir.identifier_reference(ident.span, ident.name.clone())
    }

    fn lower_private_identifier(
        &mut self,
        ident: &ast::PrivateIdentifier,
    ) -> hir::PrivateIdentifier {
        self.hir.private_identifier(ident.span, ident.name.clone())
    }

    fn lower_label_identifier(&mut self, ident: &ast::LabelIdentifier) -> hir::LabelIdentifier {
        self.hir.label_identifier(ident.span, ident.name.clone())
    }

    fn lower_identifier_name(&mut self, ident: &ast::IdentifierName) -> hir::IdentifierName {
        self.hir.identifier_name(ident.span, ident.name.clone())
    }

    fn lower_binding_identifier(
        &mut self,
        ident: &ast::BindingIdentifier,
    ) -> hir::BindingIdentifier {
        self.hir.binding_identifier(ident.span, ident.name.clone())
    }

    /* ----------  Literal ---------- */

    fn lower_number_literal(&mut self, lit: &ast::NumberLiteral<'a>) -> hir::NumberLiteral<'a> {
        let base = match lit.base {
            ast::NumberBase::Decimal => hir::NumberBase::Decimal,
            ast::NumberBase::Binary => hir::NumberBase::Binary,
            ast::NumberBase::Octal => hir::NumberBase::Octal,
            ast::NumberBase::Hex => hir::NumberBase::Hex,
        };
        self.hir.number_literal(lit.span, lit.value, lit.raw, base)
    }

    fn lower_boolean_literal(&mut self, lit: &ast::BooleanLiteral) -> hir::BooleanLiteral {
        self.hir.boolean_literal(lit.span, lit.value)
    }

    fn lower_null_literal(&mut self, lit: &ast::NullLiteral) -> hir::NullLiteral {
        self.hir.null_literal(lit.span)
    }

    fn lower_bigint_literal(&mut self, lit: &ast::BigintLiteral) -> hir::BigintLiteral {
        self.hir.bigint_literal(lit.span, lit.value.clone())
    }

    fn lower_string_literal(&mut self, lit: &ast::StringLiteral) -> hir::StringLiteral {
        self.hir.string_literal(lit.span, lit.value.clone())
    }

    fn lower_template_literal(
        &mut self,
        lit: &ast::TemplateLiteral<'a>,
    ) -> hir::TemplateLiteral<'a> {
        let quasis = self.lower_vec(&lit.quasis, Self::lower_template_element);
        let expressions = self.lower_vec(&lit.expressions, Self::lower_expression);
        self.hir.template_literal(lit.span, quasis, expressions)
    }

    fn lower_reg_expr_literal(&mut self, lit: &ast::RegExpLiteral) -> hir::RegExpLiteral {
        let flags = hir::RegExpFlags::from_bits(lit.regex.flags.bits()).unwrap();
        self.hir.reg_exp_literal(lit.span, lit.regex.pattern.clone(), flags)
    }

    fn lower_template_element(&mut self, elem: &ast::TemplateElement) -> hir::TemplateElement {
        let value = self.lower_template_element_value(&elem.value);
        self.hir.template_element(elem.span, elem.tail, value)
    }

    fn lower_template_element_value(
        &mut self,
        elem: &ast::TemplateElementValue,
    ) -> hir::TemplateElementValue {
        self.hir.template_element_value(elem.raw.clone(), elem.cooked.clone())
    }

    /* ----------  Module ---------- */

    fn lower_module_declaration(
        &mut self,
        decl: &ast::ModuleDeclaration<'a>,
    ) -> Option<hir::Statement<'a>> {
        let decl = match decl {
            ast::ModuleDeclaration::ImportDeclaration(decl) => {
                let decl = self.lower_import_declaration(decl);
                hir::ModuleDeclaration::ImportDeclaration(decl)
            }
            ast::ModuleDeclaration::ExportAllDeclaration(decl) => {
                let decl = self.lower_export_all_declaration(decl);
                hir::ModuleDeclaration::ExportAllDeclaration(decl)
            }
            ast::ModuleDeclaration::ExportDefaultDeclaration(decl) => {
                let decl = self.lower_export_default_declaration(decl)?;
                hir::ModuleDeclaration::ExportDefaultDeclaration(decl)
            }
            ast::ModuleDeclaration::ExportNamedDeclaration(decl) => {
                let decl = self.lower_export_named_declaration(decl);
                hir::ModuleDeclaration::ExportNamedDeclaration(decl)
            }
            ast::ModuleDeclaration::TSExportAssignment(_)
            | ast::ModuleDeclaration::TSNamespaceExportDeclaration(_) => {
                return None;
            }
        };
        Some(self.hir.module_declaration(decl))
    }

    fn lower_import_declaration(
        &mut self,
        decl: &ast::ImportDeclaration<'a>,
    ) -> Box<'a, hir::ImportDeclaration<'a>> {
        let specifiers = self.lower_vec(&decl.specifiers, Self::lower_import_declaration_specifier);
        let source = self.lower_string_literal(&decl.source);
        let assertions = decl
            .assertions
            .as_ref()
            .map(|attributes| self.lower_vec(attributes, Self::lower_import_attribute));
        let import_kind = match decl.import_kind {
            ast::ImportOrExportKind::Value => hir::ImportOrExportKind::Value,
            ast::ImportOrExportKind::Type => hir::ImportOrExportKind::Type,
        };
        self.hir.import_declaration(decl.span, specifiers, source, assertions, import_kind)
    }

    fn lower_import_attribute(&mut self, attribute: &ast::ImportAttribute) -> hir::ImportAttribute {
        let key = match &attribute.key {
            ast::ImportAttributeKey::Identifier(ident) => {
                let ident = self.lower_identifier_name(ident);
                hir::ImportAttributeKey::Identifier(ident)
            }
            ast::ImportAttributeKey::StringLiteral(lit) => {
                let lit = self.lower_string_literal(lit);
                hir::ImportAttributeKey::StringLiteral(lit)
            }
        };
        let value = self.lower_string_literal(&attribute.value);
        self.hir.import_attribute(attribute.span, key, value)
    }

    fn lower_import_declaration_specifier(
        &mut self,
        specifier: &ast::ImportDeclarationSpecifier,
    ) -> hir::ImportDeclarationSpecifier {
        match specifier {
            ast::ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                let specifier = self.lower_import_specifier(specifier);
                hir::ImportDeclarationSpecifier::ImportSpecifier(specifier)
            }
            ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                let specifier = self.lower_import_default_specifier(specifier);
                hir::ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier)
            }
            ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                let specifier = self.lower_import_name_specifier(specifier);
                hir::ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier)
            }
        }
    }

    fn lower_module_export_name(&mut self, name: &ast::ModuleExportName) -> hir::ModuleExportName {
        match name {
            ast::ModuleExportName::Identifier(ident) => {
                let ident = self.lower_identifier_name(ident);
                hir::ModuleExportName::Identifier(ident)
            }
            ast::ModuleExportName::StringLiteral(lit) => {
                let lit = self.lower_string_literal(lit);
                hir::ModuleExportName::StringLiteral(lit)
            }
        }
    }

    fn lower_import_specifier(&mut self, specifier: &ast::ImportSpecifier) -> hir::ImportSpecifier {
        let imported = self.lower_module_export_name(&specifier.imported);
        let local = self.lower_binding_identifier(&specifier.local);
        self.hir.import_specifier(specifier.span, imported, local)
    }

    fn lower_import_default_specifier(
        &mut self,
        specifier: &ast::ImportDefaultSpecifier,
    ) -> hir::ImportDefaultSpecifier {
        let local = self.lower_binding_identifier(&specifier.local);
        self.hir.import_default_specifier(specifier.span, local)
    }

    fn lower_import_name_specifier(
        &mut self,
        specifier: &ast::ImportNamespaceSpecifier,
    ) -> hir::ImportNamespaceSpecifier {
        let local = self.lower_binding_identifier(&specifier.local);
        self.hir.import_namespace_specifier(specifier.span, local)
    }

    fn lower_export_all_declaration(
        &mut self,
        decl: &ast::ExportAllDeclaration<'a>,
    ) -> Box<'a, hir::ExportAllDeclaration<'a>> {
        let exported = decl.exported.as_ref().map(|name| self.lower_module_export_name(name));
        let source = self.lower_string_literal(&decl.source);
        let assertions = decl
            .assertions
            .as_ref()
            .map(|attributes| self.lower_vec(attributes, Self::lower_import_attribute));
        let export_kind = match decl.export_kind {
            ast::ImportOrExportKind::Value => hir::ImportOrExportKind::Value,
            ast::ImportOrExportKind::Type => hir::ImportOrExportKind::Type,
        };
        self.hir.export_all_declaration(decl.span, exported, source, assertions, export_kind)
    }

    fn lower_export_default_declaration(
        &mut self,
        decl: &ast::ExportDefaultDeclaration<'a>,
    ) -> Option<Box<'a, hir::ExportDefaultDeclaration<'a>>> {
        let declaration = match &decl.declaration {
            ast::ExportDefaultDeclarationKind::Expression(expr) => {
                let expr = self.lower_expression(expr);
                hir::ExportDefaultDeclarationKind::Expression(expr)
            }
            ast::ExportDefaultDeclarationKind::FunctionDeclaration(decl) => {
                let decl = self.lower_function(decl);
                hir::ExportDefaultDeclarationKind::FunctionDeclaration(decl)
            }
            ast::ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                let class = self.lower_class(class);
                hir::ExportDefaultDeclarationKind::ClassDeclaration(class)
            }
            ast::ExportDefaultDeclarationKind::TSEnumDeclaration(decl) => {
                let decl = self.lower_ts_enum_declaration(decl)?;
                hir::ExportDefaultDeclarationKind::TSEnumDeclaration(decl)
            }
            ast::ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => return None,
        };
        let exported = self.lower_module_export_name(&decl.exported);
        Some(self.hir.export_default_declaration(decl.span, declaration, exported))
    }

    fn lower_export_named_declaration(
        &mut self,
        decl: &ast::ExportNamedDeclaration<'a>,
    ) -> Box<'a, hir::ExportNamedDeclaration<'a>> {
        let declaration = decl.declaration.as_ref().and_then(|decl| self.lower_declaration(decl));
        let specifiers = self.lower_vec(&decl.specifiers, Self::lower_export_specifier);
        let source = decl.source.as_ref().map(|source| self.lower_string_literal(source));
        let export_kind = match decl.export_kind {
            ast::ImportOrExportKind::Value => hir::ImportOrExportKind::Value,
            ast::ImportOrExportKind::Type => hir::ImportOrExportKind::Type,
        };
        self.hir.export_named_declaration(decl.span, declaration, specifiers, source, export_kind)
    }

    fn lower_export_specifier(&mut self, specifier: &ast::ExportSpecifier) -> hir::ExportSpecifier {
        let local = self.lower_module_export_name(&specifier.local);
        let exported = self.lower_module_export_name(&specifier.exported);
        self.hir.export_specifier(specifier.span, local, exported)
    }

    fn lower_declaration(&mut self, decl: &ast::Declaration<'a>) -> Option<hir::Declaration<'a>> {
        match decl {
            ast::Declaration::VariableDeclaration(decl) => {
                Some(hir::Declaration::VariableDeclaration(self.lower_variable_declaration(decl)))
            }
            ast::Declaration::FunctionDeclaration(func) => {
                let func = self.lower_function(func);
                Some(hir::Declaration::FunctionDeclaration(func))
            }
            ast::Declaration::ClassDeclaration(class) => {
                let class = self.lower_class(class);
                Some(hir::Declaration::ClassDeclaration(class))
            }
            ast::Declaration::TSEnumDeclaration(decl) => {
                let decl = self.lower_ts_enum_declaration(decl)?;
                Some(hir::Declaration::TSEnumDeclaration(decl))
            }
            ast::Declaration::TSImportEqualsDeclaration(decl) => {
                let decl = self.lower_ts_import_equals_declaration(decl)?;
                Some(hir::Declaration::TSImportEqualsDeclaration(decl))
            }
            _ => None,
        }
    }

    fn lower_variable_declaration(
        &mut self,
        decl: &ast::VariableDeclaration<'a>,
    ) -> Box<'a, hir::VariableDeclaration<'a>> {
        let kind = match decl.kind {
            ast::VariableDeclarationKind::Var => hir::VariableDeclarationKind::Var,
            ast::VariableDeclarationKind::Const => hir::VariableDeclarationKind::Const,
            ast::VariableDeclarationKind::Let => hir::VariableDeclarationKind::Let,
        };
        let declarations = self.lower_vec(&decl.declarations, Self::lower_variable_declarator);
        self.hir.variable_declaration(decl.span, kind, declarations)
    }

    fn lower_variable_declarator(
        &mut self,
        decl: &ast::VariableDeclarator<'a>,
    ) -> hir::VariableDeclarator<'a> {
        let kind = match decl.kind {
            ast::VariableDeclarationKind::Var => hir::VariableDeclarationKind::Var,
            ast::VariableDeclarationKind::Const => hir::VariableDeclarationKind::Const,
            ast::VariableDeclarationKind::Let => hir::VariableDeclarationKind::Let,
        };
        let id = self.lower_binding_pattern(&decl.id);
        let init = decl.init.as_ref().map(|expr| self.lower_expression(expr));
        self.hir.variable_declarator(decl.span, kind, id, init, decl.definite)
    }

    fn lower_function(&mut self, func: &ast::Function<'a>) -> Box<'a, hir::Function<'a>> {
        let r#type = match func.r#type {
            ast::FunctionType::FunctionDeclaration => hir::FunctionType::FunctionDeclaration,
            ast::FunctionType::FunctionExpression => hir::FunctionType::FunctionExpression,
            ast::FunctionType::TSDeclareFunction => hir::FunctionType::TSDeclareFunction,
        };
        let id = func.id.as_ref().map(|ident| self.lower_binding_identifier(ident));
        let params = self.lower_formal_parameters(&func.params);
        let body = func.body.as_ref().map(|body| self.lower_function_body(body));
        self.hir.function(
            r#type,
            func.span,
            id,
            func.expression,
            func.generator,
            func.r#async,
            params,
            body,
        )
    }

    fn lower_function_body(
        &mut self,
        body: &ast::FunctionBody<'a>,
    ) -> Box<'a, hir::FunctionBody<'a>> {
        let directives = self.lower_vec(&body.directives, Self::lower_directive);
        let statements = self.lower_statements(&body.statements);
        self.hir.function_body(body.span, directives, statements)
    }

    fn lower_formal_parameters(
        &mut self,
        params: &ast::FormalParameters<'a>,
    ) -> Box<'a, hir::FormalParameters<'a>> {
        let kind = match params.kind {
            ast::FormalParameterKind::FormalParameter => hir::FormalParameterKind::FormalParameter,
            ast::FormalParameterKind::UniqueFormalParameters => {
                hir::FormalParameterKind::UniqueFormalParameters
            }
            ast::FormalParameterKind::ArrowFormalParameters => {
                hir::FormalParameterKind::ArrowFormalParameters
            }
            ast::FormalParameterKind::Signature => hir::FormalParameterKind::Signature,
        };
        let items = self.lower_vec(&params.items, Self::lower_formal_parameter);
        self.hir.formal_parameters(params.span, kind, items)
    }

    fn lower_formal_parameter(
        &mut self,
        param: &ast::FormalParameter<'a>,
    ) -> hir::FormalParameter<'a> {
        let pattern = self.lower_binding_pattern(&param.pattern);
        let decorators = self.lower_vec(&param.decorators, Self::lower_decorator);
        self.hir.formal_parameter(param.span, pattern, decorators)
    }

    fn lower_class(&mut self, class: &ast::Class<'a>) -> Box<'a, hir::Class<'a>> {
        let r#type = match class.r#type {
            ast::ClassType::ClassDeclaration => hir::ClassType::ClassDeclaration,
            ast::ClassType::ClassExpression => hir::ClassType::ClassExpression,
        };
        let id = class.id.as_ref().map(|ident| self.lower_binding_identifier(ident));
        let super_class = class.super_class.as_ref().map(|expr| self.lower_expression(expr));
        let body = self.lower_class_body(&class.body);
        let decorators = self.lower_vec(&class.decorators, Self::lower_decorator);
        self.hir.class(r#type, class.span, id, super_class, body, decorators)
    }

    fn lower_class_body(&mut self, class_body: &ast::ClassBody<'a>) -> Box<'a, hir::ClassBody<'a>> {
        let mut body = self.hir.new_vec_with_capacity(class_body.body.len());
        for elem in &class_body.body {
            if let Some(elem) = self.lower_class_element(elem) {
                body.push(elem);
            }
        }
        self.hir.class_body(class_body.span, body)
    }

    fn lower_class_element(
        &mut self,
        elem: &ast::ClassElement<'a>,
    ) -> Option<hir::ClassElement<'a>> {
        match elem {
            ast::ClassElement::StaticBlock(block) => {
                let block = self.lower_static_block(block);
                Some(hir::ClassElement::StaticBlock(block))
            }
            ast::ClassElement::MethodDefinition(def) => {
                let def = self.lower_method_definition(def);
                Some(hir::ClassElement::MethodDefinition(def))
            }
            ast::ClassElement::PropertyDefinition(def) => {
                let def = self.lower_property_definition(def);
                Some(hir::ClassElement::PropertyDefinition(def))
            }
            ast::ClassElement::AccessorProperty(prop) => {
                let prop = self.lower_accessor_property(prop);
                Some(hir::ClassElement::AccessorProperty(prop))
            }
            ast::ClassElement::TSAbstractMethodDefinition(_)
            | ast::ClassElement::TSAbstractPropertyDefinition(_)
            | ast::ClassElement::TSIndexSignature(_) => None,
        }
    }

    fn lower_static_block(
        &mut self,
        block: &ast::StaticBlock<'a>,
    ) -> Box<'a, hir::StaticBlock<'a>> {
        let body = self.lower_statements(&block.body);
        self.hir.static_block(block.span, body)
    }

    fn lower_method_definition(
        &mut self,
        def: &ast::MethodDefinition<'a>,
    ) -> Box<'a, hir::MethodDefinition<'a>> {
        let key = self.lower_property_key(&def.key);
        let value = self.lower_function(&def.value);
        let kind = match def.kind {
            ast::MethodDefinitionKind::Constructor => hir::MethodDefinitionKind::Constructor,
            ast::MethodDefinitionKind::Method => hir::MethodDefinitionKind::Method,
            ast::MethodDefinitionKind::Get => hir::MethodDefinitionKind::Get,
            ast::MethodDefinitionKind::Set => hir::MethodDefinitionKind::Set,
        };
        let decorators = self.lower_vec(&def.decorators, Self::lower_decorator);
        self.hir.method_definition(
            def.span,
            key,
            value,
            kind,
            def.computed,
            def.r#static,
            def.r#override,
            def.optional,
            decorators,
        )
    }

    fn lower_property_definition(
        &mut self,
        def: &ast::PropertyDefinition<'a>,
    ) -> Box<'a, hir::PropertyDefinition<'a>> {
        let key = self.lower_property_key(&def.key);
        let value = def.value.as_ref().map(|expr| self.lower_expression(expr));
        let decorators = self.lower_vec(&def.decorators, Self::lower_decorator);
        self.hir.property_definition(
            def.span,
            key,
            value,
            def.computed,
            def.r#static,
            def.declare,
            def.r#override,
            def.optional,
            def.definite,
            def.readonly,
            decorators,
        )
    }

    fn lower_accessor_property(
        &mut self,
        def: &ast::AccessorProperty<'a>,
    ) -> Box<'a, hir::AccessorProperty<'a>> {
        let key = self.lower_property_key(&def.key);
        let value = def.value.as_ref().map(|expr| self.lower_expression(expr));
        self.hir.accessor_property(def.span, key, value, def.computed, def.r#static)
    }

    fn lower_ts_enum_declaration(
        &mut self,
        _decl: &ast::TSEnumDeclaration<'a>,
    ) -> Option<Box<'a, hir::TSEnumDeclaration<'a>>> {
        None
    }

    fn lower_ts_import_equals_declaration(
        &mut self,
        _decl: &ast::TSImportEqualsDeclaration<'a>,
    ) -> Option<Box<'a, hir::TSImportEqualsDeclaration<'a>>> {
        None
    }

    fn lower_decorator(&mut self, decorator: &ast::Decorator<'a>) -> hir::Decorator<'a> {
        let expression = self.lower_expression(&decorator.expression);
        self.hir.decorator(decorator.span, expression)
    }
}