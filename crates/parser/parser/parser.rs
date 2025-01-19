use crate::ns::*;
use lazy_regex::*;

pub struct Parser<'input> {
    tokenizer: Tokenizer<'input>,
    previous_token: (Token, Location),
    token: (Token, Location),
    locations: Vec<Location>,
    activations: Vec<ParserActivation>,
    ignore_xml_whitespace: bool,
    documentable_metadata: Vec<String>,
    expecting_token_error: bool,
}

impl<'input> Parser<'input> {
    /// Constructs a parser.
    pub fn new(compilation_unit: &'input Rc<CompilationUnit>, options: &ParserOptions) -> Self {
        Self {
            tokenizer: Tokenizer::new(compilation_unit, options),
            previous_token: (Token::Eof, Location::with_offset(&compilation_unit, 0)),
            token: (Token::Eof, Location::with_offset(&compilation_unit, 0)),
            locations: vec![],
            activations: vec![],
            ignore_xml_whitespace: options.ignore_xml_whitespace,
            documentable_metadata: options.documentable_metadata.clone(),
            expecting_token_error: false,
        }
    }

    fn options(&self) -> ParserOptions {
        ParserOptions {
            ignore_xml_whitespace: self.ignore_xml_whitespace,
            documentable_metadata: self.documentable_metadata.clone(),
            ..default()
        }
    }

    fn compilation_unit(&self) -> &Rc<CompilationUnit> {
        self.tokenizer.compilation_unit()
    }

    fn token_location(&self) -> Location {
        self.token.1.clone()
    }

    fn mark_location(&mut self) {
        self.locations.push(self.token.1.clone());
    }

    fn duplicate_location(&mut self) {
        self.locations.push(self.locations.last().unwrap().clone());
    }

    fn push_location(&mut self, location: &Location) {
        self.locations.push(location.clone());
    }

    fn pop_location(&mut self) -> Location {
        self.locations.pop().unwrap().combine_with(self.previous_token.1.clone())
    }

    fn add_syntax_error(&self, location: &Location, kind: DiagnosticKind, arguments: Vec<Rc<dyn DiagnosticArgument>>) {
        if self.compilation_unit().prevent_equal_offset_error(location) {
            return;
        }
        self.compilation_unit().add_diagnostic(Diagnostic::new_syntax_error(location, kind, arguments));
    }

    fn patch_syntax_error(&self, original: DiagnosticKind, kind: DiagnosticKind, arguments: Vec<Rc<dyn DiagnosticArgument>>) {
        if self.compilation_unit().diagnostics.borrow().is_empty() {
            return;
        }
        if self.compilation_unit().diagnostics.borrow().last().unwrap().kind == original {
            let loc = self.compilation_unit().diagnostics.borrow_mut().pop().unwrap().location();
            self.compilation_unit().add_diagnostic(Diagnostic::new_syntax_error(&loc, kind, arguments));
        }
    }

    /*
    fn add_warning(&self, location: &Location, kind: DiagnosticKind, arguments: Vec<Rc<dyn DiagnosticArgument>>) {
        if self.compilation_unit().prevent_equal_offset_warning(location) {
            return;
        }
        self.compilation_unit().add_diagnostic(Diagnostic::new_warning(location, kind, arguments));
    }
    */

    fn next(&mut self) {
        self.previous_token = self.token.clone();
        self.token = self.tokenizer.scan_ie_div();
    }

    fn next_ie_xml_tag(&mut self) {
        self.previous_token = self.token.clone();
        self.token = self.tokenizer.scan_ie_xml_tag();
    }

    fn next_ie_xml_content(&mut self) {
        self.previous_token = self.token.clone();
        self.token = self.tokenizer.scan_ie_xml_content();
    }

    fn peek(&self, token: Token) -> bool {
        self.token.0 == token
    }

    fn peek_identifier(&self, reserved_words: bool) -> Option<(String, Location)> {
        if let Token::Identifier(id) = self.token.0.clone() {
            let location = self.token.1.clone();
            Some((id, location))
        } else {
            if reserved_words {
                if let Some(id) = self.token.0.reserved_word_name() {
                    let location = self.token.1.clone();
                    return Some((id, location));
                }
            }
            None
        }
    }

    fn peek_context_keyword(&self, name: &str) -> bool {
        if let Token::Identifier(id) = self.token.0.clone() { id == name && self.token.1.character_count() == name.len() } else { false }
    }

    fn consume(&mut self, token: Token) -> bool {
        if self.token.0 == token {
            self.next();
            true
        } else {
            false
        }
    }

    fn consume_and_ie_xml_tag(&mut self, token: Token) -> bool {
        if self.token.0 == token {
            self.next_ie_xml_tag();
            true
        } else {
            false
        }
    }

    fn consume_and_ie_xml_content(&mut self, token: Token) -> bool {
        if self.token.0 == token {
            self.next_ie_xml_content();
            true
        } else {
            false
        }
    }

    fn consume_identifier(&mut self, reserved_words: bool) -> Option<(String, Location)> {
        if let Token::Identifier(id) = self.token.0.clone() {
            let location = self.token.1.clone();
            self.next();
            Some((id, location))
        } else {
            if reserved_words {
                if let Some(id) = self.token.0.reserved_word_name() {
                    let location = self.token.1.clone();
                    self.next();
                    return Some((id, location));
                }
            }
            None
        }
    }

    fn _consume_context_keyword(&mut self, name: &str) -> bool {
        if let Token::Identifier(id) = self.token.0.clone() {
            if id == name && self.token.1.character_count() == name.len() {
                self.next();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn expect(&mut self, token: Token) {
        if self.token.0 != token {
            self.expecting_token_error = true;
            self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![token.clone(), self.token.0.clone()]);
            let expecting_identifier_name = token.is_identifier_name();
            while self.token.0 != Token::Eof && (if expecting_identifier_name { self.token.0.is_identifier_name() } else { true }) {
                self.next();
                if self.token.0 == token {
                    return;
                }
            }
        } else {
            self.expecting_token_error = false;
            self.next();
        }
    }

    /// Expects a token; but if it fails, does not skip any token.
    fn non_greedy_expect(&mut self, token: Token) {
        if self.token.0 != token {
            self.expecting_token_error = true;
            self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![token.clone(), self.token.0.clone()]);
        } else {
            self.expecting_token_error = false;
            self.next();
        }
    }

    fn non_greedy_expect_virtual_semicolon(&mut self) {
        self.expecting_token_error = false;
        if !self.parse_semicolon() {
            self.expecting_token_error = true;
            self.add_syntax_error(&self.token_location(), DiagnosticKind::ExpectingEitherSemicolonOrNewLineHere, vec![]);
        }
    }

    fn expect_and_ie_xml_tag(&mut self, token: Token) {
        if self.token.0 != token {
            self.expecting_token_error = true;
            self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![token.clone(), self.token.0.clone()]);
            while self.token.0 != Token::Eof {
                self.next_ie_xml_tag();
                if self.token.0 == token {
                    return;
                }
            }
        } else {
            self.expecting_token_error = false;
            self.next_ie_xml_tag();
        }
    }

    /// Expects a token; but if it fails, does not skip any token.
    fn non_greedy_expect_and_ie_xml_tag(&mut self, token: Token) {
        if self.token.0 != token {
            self.expecting_token_error = true;
            self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![token.clone(), self.token.0.clone()]);
        } else {
            self.expecting_token_error = false;
            self.next_ie_xml_tag();
        }
    }

    fn expect_and_ie_xml_content(&mut self, token: Token) {
        if self.token.0 != token {
            self.expecting_token_error = true;
            self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![token.clone(), self.token.0.clone()]);
            while self.token.0 != Token::Eof {
                self.next_ie_xml_content();
                if self.token.0 == token {
                    return;
                }
            }
        } else {
            self.expecting_token_error = false;
            self.next_ie_xml_content();
        }
    }

    fn non_greedy_expect_and_ie_xml_content(&mut self, token: Token) {
        if self.token.0 != token {
            self.expecting_token_error = true;
            self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![token.clone(), self.token.0.clone()]);
        } else {
            self.expecting_token_error = false;
            self.next_ie_xml_content();
        }
    }

    fn expect_identifier(&mut self, reserved_words: bool) -> (String, Location) {
        if let Token::Identifier(id) = self.token.0.clone() {
            self.expecting_token_error = false;
            let location = self.token.1.clone();
            self.next();
            (id, location)
        } else {
            if reserved_words {
                if let Some(id) = self.token.0.reserved_word_name() {
                    self.expecting_token_error = false;
                    let location = self.token.1.clone();
                    self.next();
                    return (id, location);
                }
            }
            self.expecting_token_error = true;
            self.add_syntax_error(&self.token_location(), DiagnosticKind::ExpectingIdentifier, diagarg![self.token.0.clone()]);
            /*
            while self.token.0 != Token::Eof && self.token.0.is_identifier_name() {
                if let Some(id) = self.consume_identifier(reserved_words) {
                    return id;
                } else {
                    self.next();
                }
            }
            */
            (INVALIDATED_IDENTIFIER.to_owned(), self.tokenizer.cursor_location())
        }
    }

    fn _expect_context_keyword(&mut self, name: &str) {
        if let Token::Identifier(id) = self.token.0.clone() {
            if id == name && self.token.1.character_count() == name.len() {
                self.expecting_token_error = false;
                self.next();
                return;
            }
        }
        self.expecting_token_error = true;
        self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![format!("'{name}'"), self.token.0.clone()]);
        while self.token.0 != Token::Eof && self.token.0.is_identifier_name() {
            if self._consume_context_keyword(name) {
                return;
            } else {
                self.next();
            }
        }
    }

    fn non_greedy_expect_context_keyword(&mut self, name: &str) {
        if let Token::Identifier(id) = self.token.0.clone() {
            if id == name && self.token.1.character_count() == name.len() {
                self.expecting_token_error = false;
                self.next();
                return;
            }
        }
        self.expecting_token_error = true;
        self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![format!("'{name}'"), self.token.0.clone()]);
    }

    /// Expects a greater-than symbol. If the facing token is not greater-than,
    /// but starts with a greater-than symbol, the first character is shifted off
    /// from the facing token.
    fn _expect_type_parameters_gt(&mut self) {
        self.expecting_token_error = false;
        if !self.consume_type_parameters_gt() {
            self.expecting_token_error = true;
            self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![Token::Gt, self.token.0.clone()]);
            while self.token.0 != Token::Eof {
                self.next();
                if self.consume_type_parameters_gt() {
                    return;
                }
            }
        }
    }

    fn non_greedy_expect_type_parameters_gt(&mut self) {
        self.expecting_token_error = false;
        if !self.consume_type_parameters_gt() {
            self.expecting_token_error = true;
            self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![Token::Gt, self.token.0.clone()]);
        }
    }

    /// Consumes a greater-than symbol. If the facing token is not greater-than,
    /// but starts with a greater-than symbol, the first character is shifted off
    /// from the facing token.
    fn consume_type_parameters_gt(&mut self) -> bool {
        match self.token.0 {
            Token::Gt => {
                self.next();
                true
            },
            Token::Ge => {
                self.token.0 = Token::Assign;
                self.token.1.first_offset += 1;
                true
            },
            Token::RightShift => {
                self.token.0 = Token::Gt;
                self.token.1.first_offset += 1;
                true
            },
            Token::RightShiftAssign => {
                self.token.0 = Token::Ge;
                self.token.1.first_offset += 1;
                true
            },
            Token::UnsignedRightShift => {
                self.token.0 = Token::RightShift;
                self.token.1.first_offset += 1;
                true
            },
            Token::UnsignedRightShiftAssign => {
                self.token.0 = Token::RightShiftAssign;
                self.token.1.first_offset += 1;
                true
            },
            _ => {
                false
            },
        }
    }

    fn offending_token_is_inline_or_higher_indented(&self) -> bool {
        if !self.previous_token.1.line_break(&self.token.1) {
            return true;
        }
        let i1 = self.compilation_unit().get_line_indent(self.previous_token.1.first_line_number());
        let i2 = self.compilation_unit().get_line_indent(self.token.1.first_line_number());
        i2 > i1
    }

    pub fn expect_eof(&mut self) {
        self.expect(Token::Eof)
    }

    fn create_invalidated_expression(&self, location: &Location) -> Rc<Expression> {
        Rc::new(Expression::Invalidated(InvalidatedNode {
            location: location.clone(),
        }))
    }

    fn create_invalidated_directive(&self, location: &Location) -> Rc<Directive> {
        Rc::new(Directive::Invalidated(InvalidatedNode {
            location: location.clone(),
        }))
    }

    pub fn parse_metadata(&mut self) -> (Vec<Attribute>, Option<Rc<Asdoc>>) {
        let Some(exp) = self.parse_opt_expression(Default::default()) else {
            return (vec![], self.parse_asdoc());
        };
        self.expect(Token::Eof);

        match exp.to_metadata(self) {
            Ok(Some(metadata)) => {
                // For meta-data that are not one of certain Flex meta-data,
                // delegate the respective ASDoc forward.
                let mut new_metadata = Vec::<Attribute>::new();
                let mut asdoc: Option<Rc<Asdoc>> = None;
                for attr in &metadata {
                    if let Attribute::Metadata(metadata) = attr {
                        if !self.documentable_metadata.contains(&metadata.name.0) && metadata.asdoc.is_some() {
                            new_metadata.push(Attribute::Metadata(Rc::new(Metadata {
                                location: metadata.location.clone(),
                                asdoc: None,
                                name: metadata.name.clone(),
                                entries: metadata.entries.clone(),
                            })));
                            asdoc = metadata.asdoc.clone();
                        } else {
                            new_metadata.push(attr.clone());
                        }
                    } else {
                        new_metadata.push(attr.clone());
                    }
                }

                (new_metadata, asdoc)
            },
            Ok(None) => {
                self.add_syntax_error(&exp.location(), DiagnosticKind::UnallowedExpression, diagarg![]);
                (vec![], None)
            },
            Err(MetadataRefineError1(MetadataRefineError::Syntax, loc)) => {
                let asdoc = self.parse_asdoc();
                self.add_syntax_error(&loc, DiagnosticKind::UnrecognizedMetadataSyntax, diagarg![]);
                (vec![], asdoc)
            },
        }
    }

    pub fn parse_metadata_content(&mut self) -> Rc<Metadata> {
        let loc1 = self.token.1.clone();
        let Some(exp) = self.parse_opt_expression(Default::default()) else {
            self.push_location(&loc1);
            self.expect_identifier(false);
            return Rc::new(Metadata {
                location: self.pop_location(),
                asdoc: None,
                name: (INVALIDATED_IDENTIFIER.to_owned(), loc1),
                entries: None,
            });
        };
        self.expect(Token::Eof);

        match self.refine_metadata(&exp, None) {
            Ok(metadata) => {
                metadata
            },
            Err(MetadataRefineError::Syntax) => {
                self.push_location(&loc1);
                self.add_syntax_error(&exp.location(), DiagnosticKind::UnrecognizedMetadataSyntax, diagarg![]);
                Rc::new(Metadata {
                    location: self.pop_location(),
                    asdoc: None,
                    name: (INVALIDATED_IDENTIFIER.to_owned(), loc1),
                    entries: None,
                })
            },
        }
    }

    pub fn parse_expression(&mut self, context: ParserExpressionContext) -> Rc<Expression> {
        if let Some(exp) = self.parse_opt_expression(context) {
            exp
        } else {
            self.add_syntax_error(&self.token_location(), DiagnosticKind::ExpectingExpression, diagarg![self.token.0.clone()]);
            self.create_invalidated_expression(&self.tokenizer.cursor_location())
        }
    }

    pub fn parse_opt_expression(&mut self, context: ParserExpressionContext) -> Option<Rc<Expression>> {
        let exp: Option<Rc<Expression>> = self.parse_opt_start_expression(context.clone());

        // Parse subexpressions
        if let Some(exp) = exp {
            return Some(self.parse_subexpressions(exp, context.clone()));
        }
        None
    }

    fn parse_subexpressions(&mut self, mut base: Rc<Expression>, context: ParserExpressionContext) -> Rc<Expression> {
        loop {
            if self.consume(Token::Dot) {
                base = self.parse_dot_subexpression(base);
            } else if self.consume(Token::OptionalChaining) {
                base = self.parse_optional_chaining(base);
            } else if self.peek(Token::SquareOpen) {
                let asdoc = self.parse_asdoc();
                self.next();
                self.push_location(&base.location());
                let key = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
                self.non_greedy_expect(Token::SquareClose);
                base = Rc::new(Expression::ComputedMember(ComputedMemberExpression {
                    base, asdoc, key, location: self.pop_location()
                }));
            } else if self.consume(Token::Descendants) {
                self.push_location(&base.location());
                let id = self.parse_qualified_identifier();
                base = Rc::new(Expression::Descendants(DescendantsExpression {
                    location: self.pop_location(),
                    base,
                    identifier: id,
                }));
            } else if self.peek(Token::ParenOpen) {
                self.push_location(&base.location());
                let arguments = self.parse_arguments();
                base = Rc::new(Expression::Call(CallExpression {
                    location: self.pop_location(),
                    base,
                    arguments,
                }));
            } else if self.peek(Token::Increment) && !self.previous_token.1.line_break(&self.token.1) {
                self.push_location(&base.location());
                self.next();
                base = Rc::new(Expression::Unary(UnaryExpression {
                    location: self.pop_location(),
                    expression: base,
                    operator: Operator::PostIncrement,
                }));
            } else if self.peek(Token::Decrement) && !self.previous_token.1.line_break(&self.token.1) {
                self.push_location(&base.location());
                self.next();
                base = Rc::new(Expression::Unary(UnaryExpression {
                    location: self.pop_location(),
                    expression: base,
                    operator: Operator::PostDecrement,
                }));
            } else if self.peek(Token::Exclamation) && !self.previous_token.1.line_break(&self.token.1) {
                self.push_location(&base.location());
                self.next();
                base = Rc::new(Expression::Unary(UnaryExpression {
                    location: self.pop_location(),
                    expression: base, operator: Operator::NonNull,
                }));
            // `not in`
            } else if self.token.0 == Token::Not && context.allow_in && context.min_precedence.includes(&OperatorPrecedence::Relational) && !self.previous_token.1.line_break(&self.token.1) {
                self.push_location(&base.location());
                self.next();
                self.non_greedy_expect(Token::In);
                base = self.parse_binary_operator(base, Operator::NotIn, OperatorPrecedence::Relational.add(1).unwrap(), context.clone());
            // ConditionalExpression
            } else if self.peek(Token::Question) && context.min_precedence.includes(&OperatorPrecedence::AssignmentAndOther) {
                self.push_location(&base.location());
                self.next();
                let consequent = self.parse_expression(ParserExpressionContext {
                    min_precedence: OperatorPrecedence::AssignmentAndOther,
                    ..context.clone()
                });
                let mut alternative = self.create_invalidated_expression(&self.tokenizer.cursor_location());
                self.non_greedy_expect(Token::Colon);
                if !self.expecting_token_error {
                    alternative = self.parse_expression(ParserExpressionContext {
                        min_precedence: OperatorPrecedence::AssignmentAndOther,
                        ..context.clone()
                    });
                }
                base = Rc::new(Expression::Conditional(ConditionalExpression {
                    location: self.pop_location(),
                    test: base, consequent, alternative,
                }));
            } else if let Some(binary_operator) = self.check_binary_operator(context.clone()) {
                let BinaryOperator(operator, required_precedence, _) = binary_operator;
                if context.min_precedence.includes(&required_precedence) {
                    self.next();
                    base = self.parse_binary_operator(base, operator, binary_operator.right_precedence(), context.clone());
                } else {
                    break;
                }
            // AssignmentExpression
            } else if self.peek(Token::Assign) && context.min_precedence.includes(&OperatorPrecedence::AssignmentAndOther) && context.allow_assignment {
                self.push_location(&base.location());
                self.next();
                let left = base.clone();
                if !left.is_valid_assignment_left_hand_side() {
                    self.add_syntax_error(&left.location(), DiagnosticKind::MalformedDestructuring, vec![])
                }
                let right = self.parse_expression(ParserExpressionContext {
                    min_precedence: OperatorPrecedence::AssignmentAndOther,
                    ..context.clone()
                });
                base = Rc::new(Expression::Assignment(AssignmentExpression {
                    location: self.pop_location(),
                    left, compound: None, right,
                }));
            // CompoundAssignment and LogicalAssignment
            } else if let Some(compound) = self.token.0.compound_assignment() {
                if context.min_precedence.includes(&OperatorPrecedence::AssignmentAndOther) && context.allow_assignment {
                    self.push_location(&base.location());
                    self.next();
                    let left = base.clone();
                    let right = self.parse_expression(ParserExpressionContext {
                        min_precedence: OperatorPrecedence::AssignmentAndOther,
                        ..context.clone()
                    });
                    base = Rc::new(Expression::Assignment(AssignmentExpression {
                        location: self.pop_location(),
                        left, compound: Some(compound), right,
                    }));
                } else {
                    break;
                }
            } else if self.peek(Token::Comma) && context.min_precedence.includes(&OperatorPrecedence::List) {
                self.push_location(&base.location());
                self.next();
                let right = self.parse_expression(ParserExpressionContext {
                    min_precedence: OperatorPrecedence::AssignmentAndOther,
                    ..context.clone()
                });
                base = Rc::new(Expression::Sequence(SequenceExpression {
                    location: self.pop_location(),
                    left: base, right,
                }));
            } else {
                break;
            }
        }

        base
    }

    fn parse_binary_operator(&mut self, base: Rc<Expression>, mut operator: Operator, right_precedence: OperatorPrecedence, context: ParserExpressionContext) -> Rc<Expression> {
        // The left operand of a null-coalescing operation must not be
        // a logical AND, XOR or OR operation.
        if operator == Operator::NullCoalescing {
            if let Expression::Unary(UnaryExpression { expression, operator, .. }) = base.as_ref() {
                if [Operator::LogicalAnd, Operator::LogicalXor, Operator::LogicalOr].contains(&operator) {
                    self.add_syntax_error(&expression.location(), DiagnosticKind::IllegalNullishCoalescingLeftOperand, vec![]);
                }
            }
        }

        if operator == Operator::Is && self.consume(Token::Not) {
            operator = Operator::IsNot;
        }

        self.push_location(&base.location());
        let right = self.parse_expression(ParserExpressionContext {
            min_precedence: right_precedence,
            ..context
        });
        Rc::new(Expression::Binary(BinaryExpression {
            location: self.pop_location(),
            left: base, operator, right,
        }))
    }

    fn check_binary_operator(&self, context: ParserExpressionContext) -> Option<BinaryOperator> {
        if let Some(operator) = self.token.0.to_binary_operator() {
            if operator == Operator::In && !context.allow_in {
                return None;
            }
            BinaryOperator::try_from(operator).ok()
        } else {
            None
        }
    }

    fn parse_optional_chaining(&mut self, base: Rc<Expression>) -> Rc<Expression> {
        self.push_location(&base.location());
        self.duplicate_location();
        let mut operation = Rc::new(Expression::OptionalChainingPlaceholder(OptionalChainingPlaceholder {
            location: base.location(),
        }));
        if self.peek(Token::ParenOpen) {
            let arguments: Vec<Rc<Expression>> = self.parse_arguments();
            if arguments.len() == 1 && self.peek(Token::ColonColon) {
                self.duplicate_location();
                let ql = self.pop_location();
                let q = Rc::new(Expression::Paren(ParenExpression {
                    location: ql.clone(),
                    expression: arguments[0].clone(),
                }));
                let identifier = self.finish_qualified_identifier(false, ql, q);
                operation = Rc::new(Expression::Member(MemberExpression {
                    location: self.pop_location(),
                    base: operation,
                    identifier,
                }));
            } else {
                operation = Rc::new(Expression::Call(CallExpression {
                    location: self.pop_location(),
                    base: operation, arguments
                }));
            }
        } else if self.peek(Token::SquareOpen) {
            let asdoc = self.parse_asdoc();
            self.next();
            let key = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
            self.non_greedy_expect(Token::SquareClose);
            operation = Rc::new(Expression::ComputedMember(ComputedMemberExpression {
                location: self.pop_location(),
                base: operation, asdoc, key,
            }));
        } else {
            let identifier = self.parse_qualified_identifier();
            operation = Rc::new(Expression::Member(MemberExpression {
                location: self.pop_location(),
                base: operation, identifier
            }));
        }

        // Parse postfix subexpressions
        operation = self.parse_optional_chaining_subexpressions(operation);

        Rc::new(Expression::OptionalChaining(OptionalChainingExpression {
            location: self.pop_location(),
            base, expression: operation,
        }))
    }

    fn parse_optional_chaining_subexpressions(&mut self, mut base: Rc<Expression>) -> Rc<Expression> {
        loop {
            if self.consume(Token::Dot) {
                base = self.parse_dot_subexpression(base);
            } else if self.consume(Token::OptionalChaining) {
                base = self.parse_optional_chaining(base);
            } else if self.peek(Token::SquareOpen) {
                self.next();
                self.push_location(&base.location());
                let key = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
                self.non_greedy_expect(Token::SquareClose);
                base = Rc::new(Expression::ComputedMember(ComputedMemberExpression {
                    base, asdoc: None, key, location: self.pop_location()
                }));
            } else if self.consume(Token::Descendants) {
                self.push_location(&base.location());
                let id = self.parse_qualified_identifier();
                base = Rc::new(Expression::Descendants(DescendantsExpression {
                    location: self.pop_location(),
                    base,
                    identifier: id,
                }));
            } else if self.peek(Token::ParenOpen) {
                self.push_location(&base.location());
                let arguments = self.parse_arguments();
                base = Rc::new(Expression::Call(CallExpression {
                    location: self.pop_location(),
                    base,
                    arguments,
                }));
            } else if self.peek(Token::Exclamation) && !self.previous_token.1.line_break(&self.token.1) {
                self.push_location(&base.location());
                self.next();
                base = Rc::new(Expression::Unary(UnaryExpression {
                    location: self.pop_location(),
                    expression: base, operator: Operator::NonNull,
                }));
            } else {
                break;
            }
        }

        base
    }

    fn parse_dot_subexpression(&mut self, base: Rc<Expression>) -> Rc<Expression> {
        self.push_location(&base.location());
        if self.peek(Token::ParenOpen) {
            let paren_location = self.token_location();
            let paren_exp = self.parse_paren_list_expression();
            if !matches!(paren_exp.as_ref(), Expression::Sequence(_)) && self.peek(Token::ColonColon) {
                let q = Rc::new(Expression::Paren(ParenExpression {
                    location: paren_location.clone(),
                    expression: paren_exp.clone(),
                }));
                let id = self.finish_qualified_identifier(false, paren_location, q);
                Rc::new(Expression::Member(MemberExpression {
                    location: self.pop_location(),
                    base, identifier: id
                }))
            } else {
                Rc::new(Expression::Filter(FilterExpression {
                    location: self.pop_location(),
                    base, test: paren_exp
                }))
            }
        } else if self.consume(Token::Lt) {
            let mut arguments = vec![];
            arguments.push(self.parse_type_expression());
            while self.consume(Token::Comma) {
                arguments.push(self.parse_type_expression());
            }
            self.non_greedy_expect_type_parameters_gt();
            Rc::new(Expression::WithTypeArguments(ApplyTypeExpression {
                location: self.pop_location(),
                base, arguments
            }))
        } else {
            let id = self.parse_qualified_identifier();
            Rc::new(Expression::Member(MemberExpression {
                location: self.pop_location(),
                base, identifier: id
            }))
        }
    }

    /// Ensures a parameter list consists of zero or more required parameters followed by
    /// zero or more optional parameters optionally followed by a rest parameter.
    fn validate_parameter_list(&mut self, params: Vec<(ParameterKind, Location)>) {
        let mut least_kind = ParameterKind::Required; 
        let mut has_rest = false;
        for (param_kind, param_loc) in params {
            if !least_kind.may_be_followed_by(param_kind) {
                self.add_syntax_error(&param_loc, DiagnosticKind::WrongParameterPosition, vec![]);
            }
            least_kind = param_kind;
            if param_kind == ParameterKind::Rest && has_rest {
                self.add_syntax_error(&param_loc, DiagnosticKind::DuplicateRestParameter, vec![]);
            }
            has_rest = param_kind == ParameterKind::Rest;
        }
    }

    fn parse_opt_start_expression(&mut self, context: ParserExpressionContext) -> Option<Rc<Expression>> {
        if let Token::Identifier(id) = self.token.0.clone() {
            let id_location = self.token_location();
            self.next();
            Some(self.parse_expression_starting_with_identifier((id, id_location)))
        } else if self.peek(Token::Null) {
            self.mark_location();
            self.next();
            Some(Rc::new(Expression::NullLiteral(NullLiteral {
                location: self.pop_location(),
            })))
        } else if self.peek(Token::False) {
            self.mark_location();
            self.next();
            Some(Rc::new(Expression::BooleanLiteral(BooleanLiteral {
                location: self.pop_location(),
                value: false,
            })))
        } else if self.peek(Token::True) {
            self.mark_location();
            self.next();
            Some(Rc::new(Expression::BooleanLiteral(BooleanLiteral {
                location: self.pop_location(),
                value: true,
            })))
        } else if let Token::Number(n, suffix) = self.token.0.clone() {
            self.mark_location();
            self.next();
            Some(Rc::new(Expression::NumericLiteral(NumericLiteral {
                location: self.pop_location(),
                value: n,
                suffix,
            })))
        } else if let Token::String(ref s) = self.token.0.clone() {
            self.mark_location();
            self.next();
            Some(Rc::new(Expression::StringLiteral(StringLiteral {
                location: self.pop_location(),
                value: s.clone(),
            })))
        } else if self.peek(Token::This) {
            self.mark_location();
            self.next();
            Some(Rc::new(Expression::ThisLiteral(ThisLiteral {
                location: self.pop_location(),
            })))
        } else if self.peek(Token::Div) || self.peek(Token::DivideAssign) {
            self.mark_location();
            self.token = self.tokenizer.scan_regexp_literal(self.token.1.clone(), if self.peek(Token::DivideAssign) { "=".into() } else { "".into() });
            let Token::RegExp { ref body, ref flags } = self.token.0.clone() else {
                panic!();
            };
            self.next();
            Some(Rc::new(Expression::RegExpLiteral(RegExpLiteral {
                location: self.pop_location(),
                body: body.clone(), flags: flags.clone(),
            })))
        // `@`
        } else if self.peek(Token::Attribute) {
            self.mark_location();
            let id = self.parse_qualified_identifier();
            Some(Rc::new(Expression::QualifiedIdentifier(id)))
        // Parentheses
        } else if self.peek(Token::ParenOpen) {
            Some(self.parse_paren_list_expr_or_qual_id())
        // XMLList, XMLElement, XMLMarkup
        } else if self.peek(Token::Lt) {
            if let Some(token) = self.tokenizer.scan_xml_markup(self.token_location()) {
                self.token = token;
            }
            let start = self.token_location();
            if let Token::XmlMarkup(content) = &self.token.0.clone() {
                self.mark_location();
                self.next();
                Some(Rc::new(Expression::XmlMarkup(XmlMarkupExpression {
                    location: self.pop_location(),
                    markup: content.clone(),
                })))
            } else {
                Some(self.parse_xml_element_or_xml_list(start))
            }
        // ArrayInitializer
        } else if self.peek(Token::SquareOpen) {
            Some(self.parse_array_initializer())
        // NewExpression
        } else if self.peek(Token::New) && context.min_precedence.includes(&OperatorPrecedence::Unary) {
            let start = self.token_location();
            self.next();
            Some(self.parse_new_expression(start))
        } else if self.peek(Token::BlockOpen) {
            Some(self.parse_object_initializer())
        } else if self.peek(Token::Function) && context.min_precedence.includes(&OperatorPrecedence::AssignmentAndOther) {
            Some(self.parse_function_expression(context.clone()))
        // SuperExpression
        } else if self.peek(Token::Super) && context.min_precedence.includes(&OperatorPrecedence::Postfix) {
            Some(self.parse_super_expression_followed_by_property_operator())
        // AwaitExpression
        } else if self.peek(Token::Await) && context.min_precedence.includes(&OperatorPrecedence::Unary) {
            self.mark_location();
            let operator_token = self.token.clone();
            self.next();
            let base = self.parse_expression(ParserExpressionContext {
                allow_in: true,
                min_precedence: OperatorPrecedence::Unary,
                ..default()
            });
            if let Some(activation) = self.activations.last_mut() {
                activation.uses_await = true;
            } else {
                self.add_syntax_error(&operator_token.1, DiagnosticKind::NotAllowedHere, diagarg![operator_token.0]);
            }
            Some(Rc::new(Expression::Unary(UnaryExpression {
                location: self.pop_location(),
                expression: base, operator: Operator::Await,
            })))
        // YieldExpression
        } else if self.peek(Token::Yield) && context.min_precedence.includes(&OperatorPrecedence::AssignmentAndOther) {
            self.mark_location();
            let operator_token = self.token.clone();
            self.next();
            let base = self.parse_expression(ParserExpressionContext {
                allow_in: true,
                min_precedence: OperatorPrecedence::AssignmentAndOther,
                ..default()
            });
            if let Some(activation) = self.activations.last_mut() {
                activation.uses_yield = true;
            } else {
                self.add_syntax_error(&operator_token.1, DiagnosticKind::NotAllowedHere, diagarg![operator_token.0]);
            }
            Some(Rc::new(Expression::Unary(UnaryExpression {
                location: self.pop_location(),
                expression: base, operator: Operator::Yield,
            })))
        // Miscellaneous prefix unary expressions
        } else if let Some((operator, subexp_precedence)) = self.check_prefix_operator() {
            if context.min_precedence.includes(&OperatorPrecedence::Unary) {
                self.mark_location();
                self.next();
                let base = self.parse_expression(ParserExpressionContext { min_precedence: subexp_precedence, ..default() });
                Some(Rc::new(Expression::Unary(UnaryExpression {
                    location: self.pop_location(),
                    expression: base, operator,
                })))
            } else {
                None
            }
        // ImportMeta
        } else if self.peek(Token::Import) && context.min_precedence.includes(&OperatorPrecedence::Postfix) {
            self.mark_location();
            self.next();
            self.non_greedy_expect(Token::Dot);
            self.non_greedy_expect_context_keyword("meta");
            Some(Rc::new(Expression::ImportMeta(ImportMeta {
                location: self.pop_location(),
            })))
        // QualifiedIdentifier
        } else if
                self.peek(Token::Times)
            ||  self.peek(Token::Public) || self.peek(Token::Private)
            ||  self.peek(Token::Protected) || self.peek(Token::Internal) {
            let id = self.parse_qualified_identifier();
            Some(Rc::new(Expression::QualifiedIdentifier(id)))
        } else {
            None
        }
    }

    fn parse_expression_starting_with_identifier(&mut self, id: (String, Location)) -> Rc<Expression> {
        let id_location = id.1.clone();
        let id = id.0;

        /*
        // EmbedExpression
        if self.peek(Token::BlockOpen) && id == "embed" && self.previous_token.1.character_count() == "embed".len() {
            return self.finish_embed_expression(id_location);
        }
        */

        let id = Rc::new(Expression::QualifiedIdentifier(QualifiedIdentifier {
            location: id_location.clone(),
            attribute: false,
            qualifier: None,
            id: QualifiedIdentifierIdentifier::Id((id, id_location.clone())),
        }));
        if self.peek(Token::ColonColon) {
            self.push_location(&id_location.clone());
            let ql = self.pop_location();
            let id = self.finish_qualified_identifier(false, ql, id);
            Rc::new(Expression::QualifiedIdentifier(id))
        } else {
            id
        }
    }

    fn check_prefix_operator(&self) -> Option<(Operator, OperatorPrecedence)> {
        match self.token.0 {
            Token::Delete => Some((Operator::Delete, OperatorPrecedence::Postfix)),
            Token::Void => Some((Operator::Void, OperatorPrecedence::Unary)),
            Token::Typeof => Some((Operator::Typeof, OperatorPrecedence::Unary)),
            Token::Increment => Some((Operator::PreIncrement, OperatorPrecedence::Postfix)),
            Token::Decrement => Some((Operator::PreDecrement, OperatorPrecedence::Postfix)),
            Token::Plus => Some((Operator::Positive, OperatorPrecedence::Unary)),
            Token::Minus => Some((Operator::Negative, OperatorPrecedence::Unary)),
            Token::Tilde => Some((Operator::BitwiseNot, OperatorPrecedence::Unary)),
            Token::Exclamation => Some((Operator::LogicalNot, OperatorPrecedence::Unary)),
            _ => None,
        }
    }

    fn parse_function_expression(&mut self, context: ParserExpressionContext) -> Rc<Expression> {
        self.mark_location();
        self.next();
        let mut name = None;
        if let Token::Identifier(id) = self.token.0.clone() {
            name = Some((id, self.token.1.clone()));
            self.next();
        }
        let common = self.parse_function_common(true, ParserDirectiveContext::Default, context.allow_in);
        Rc::new(Expression::Function(FunctionExpression {
            location: self.pop_location(),
            name,
            common,
        }))
    }

    fn parse_function_common(&mut self, function_expr: bool, block_context: ParserDirectiveContext, allow_in: bool) -> Rc<FunctionCommon> {
        self.mark_location();
        self.duplicate_location();
        let mut this_parameter: Option<Rc<ThisParameter>> = None;
        let mut params: Vec<Rc<Parameter>> = vec![];
        let mut return_annotation = Some(self.create_invalidated_expression(&self.tokenizer.cursor_location()));
        self.non_greedy_expect(Token::ParenOpen);
        if !self.expecting_token_error {
            if !self.peek(Token::ParenClose) {
                if self.peek(Token::This) {
                    self.mark_location();
                    self.next();
                    let mut type_annotation = self.create_invalidated_expression(&self.tokenizer.cursor_location());
                    self.expect(Token::Colon);
                    if !self.expecting_token_error
                    {
                        type_annotation = self.parse_type_expression();
                    }
                    this_parameter = Some(Rc::new(ThisParameter {
                        location: self.pop_location(),
                        type_annotation,
                    }));
                } else {
                    params.push(self.parse_parameter());
                }
                while self.consume(Token::Comma) {
                    params.push(self.parse_parameter());
                }
            }
            self.non_greedy_expect(Token::ParenClose);
            if !self.expecting_token_error {
                return_annotation = if self.consume(Token::Colon) { Some(self.parse_type_expression()) } else { None };
            }
            self.validate_parameter_list(params.iter().map(|p| (p.kind, p.location.clone())).collect::<Vec<_>>());
        }

        let signature_location = self.pop_location();

        // Enter activation
        self.activations.push(ParserActivation::new());

        // Body
        let body = if self.peek(Token::BlockOpen) {
            Some(FunctionBody::Block(Rc::new(self.parse_block(block_context))))
        } else if !(self.offending_token_is_inline_or_higher_indented() || self.peek(Token::ParenOpen)) {
            None
        } else {
            self.parse_opt_expression(ParserExpressionContext {
                allow_in,
                min_precedence: OperatorPrecedence::AssignmentAndOther,
                ..default()
            }).map(|e| FunctionBody::Expression(e))
        };

        // Body is required by function expressions
        if body.is_none() && function_expr {
            self.non_greedy_expect(Token::BlockOpen);
        }

        // Exit activation
        let activation = self.activations.pop().unwrap();

        Rc::new(FunctionCommon {
            location: self.pop_location(),
            contains_await: activation.uses_await,
            contains_yield: activation.uses_yield,
            signature: FunctionSignature {
                location: signature_location,
                this_parameter,
                parameters: params,
                result_type: return_annotation,
            },
            body,
        })
    }

    fn parse_parameter(&mut self) -> Rc<Parameter> {
        self.mark_location();
        let rest = self.consume(Token::Ellipsis);
        let binding: Rc<VariableBinding> = Rc::new(self.parse_variable_binding(true));
        let has_initializer = binding.initializer.is_some();
        let location = self.pop_location();
        if rest && has_initializer {
            self.add_syntax_error(&location.clone(), DiagnosticKind::MalformedRestParameter, vec![]);
        }
        Rc::new(Parameter {
            location,
            destructuring: binding.destructuring.clone(),
            default_value: binding.initializer.clone(),
            kind: if rest {
                ParameterKind::Rest
            } else if has_initializer {
                ParameterKind::Optional
            } else {
                ParameterKind::Required
            },
        })
    }

    fn parse_object_initializer(&mut self) -> Rc<Expression> {
        self.mark_location();
        self.non_greedy_expect(Token::BlockOpen);
        let mut fields: Vec<Rc<InitializerField>> = vec![];
        while !self.peek(Token::BlockClose) {
            fields.push(self.parse_field());
            if !self.consume(Token::Comma) {
                break;
            }
        }
        self.non_greedy_expect(Token::BlockClose);

        Rc::new(Expression::ObjectInitializer(ObjectInitializer {
            location: self.pop_location(),
            fields,
        }))
    }

    fn parse_field(&mut self) -> Rc<InitializerField> {
        if self.peek(Token::Ellipsis) {
            self.mark_location();
            self.next();
            let subexp = self.parse_expression(ParserExpressionContext {
                allow_in: true,
                min_precedence: OperatorPrecedence::AssignmentAndOther,
                ..default()
            });
            return Rc::new(InitializerField::Rest((subexp, self.pop_location())));
        }

        let name = self.parse_field_name();

        let non_null = self.consume(Token::Exclamation);
        let mut value = None;

        if self.consume(Token::Colon) {
            value = Some(self.parse_expression(ParserExpressionContext {
                allow_in: true,
                min_precedence: OperatorPrecedence::AssignmentAndOther,
                ..default()
            }));
        } else if !matches!(name.0, FieldName::Identifier(_)) {
            self.non_greedy_expect(Token::Colon);
        }

        Rc::new(InitializerField::Field {
            name,
            non_null,
            value,
        })
    }

    fn parse_field_name(&mut self) -> (FieldName, Location) {
        if let Token::String(value) = &self.token.0.clone() {
            let location = self.token_location();
            self.next();
            (FieldName::StringLiteral(Rc::new(Expression::StringLiteral(StringLiteral {
                location: location.clone(),
                value: value.clone(),
            }))), location)
        } else if let Token::Number(value, suffix) = &self.token.0.clone() {
            let location = self.token_location();
            self.next();
            (FieldName::NumericLiteral(Rc::new(Expression::NumericLiteral(NumericLiteral {
                location: location.clone(),
                value: value.clone(),
                suffix: *suffix,
            }))), location)
        } else if self.peek(Token::SquareOpen) {
            self.mark_location();
            self.next();
            let key_expr = self.parse_expression(ParserExpressionContext {
                allow_in: true,
                min_precedence: OperatorPrecedence::List,
                ..default()
            });
            self.non_greedy_expect(Token::SquareClose);
            let location = self.pop_location();
            (FieldName::Brackets(key_expr), location)
        } else {
            let id = self.parse_non_attribute_qualified_identifier();
            let l = id.location.clone();
            (FieldName::Identifier(id), l)
        }
    }

    fn parse_new_expression(&mut self, start: Location) -> Rc<Expression> {
        self.push_location(&start);
        if self.consume(Token::Lt) {
            let element_type = self.parse_type_expression();
            self.non_greedy_expect_type_parameters_gt();
            let mut elements: Vec<Element> = vec![];
            self.non_greedy_expect(Token::SquareOpen);
            if !self.expecting_token_error {
                while !self.peek(Token::SquareClose) {
                    if self.peek(Token::Ellipsis) {
                        self.mark_location();
                        self.next();
                        elements.push(Element::Rest((self.parse_expression(ParserExpressionContext {
                            allow_in: true,
                            min_precedence: OperatorPrecedence::AssignmentAndOther,
                            ..default()
                        }), self.pop_location())));
                    } else {
                        elements.push(Element::Expression(self.parse_expression(ParserExpressionContext {
                            allow_in: true,
                            min_precedence: OperatorPrecedence::AssignmentAndOther,
                            ..default()
                        })));
                    }
                    if !self.consume(Token::Comma) {
                        break;
                    }
                }
                self.non_greedy_expect(Token::SquareClose);
            }
            Rc::new(Expression::VectorLiteral(VectorLiteral {
                location: self.pop_location(),
                element_type,
                elements,
            }))
        } else {
            let base = self.parse_new_subexpression();
            let arguments = if self.peek(Token::ParenOpen) { Some(self.parse_arguments()) } else { None };
            Rc::new(Expression::New(NewExpression {
                location: self.pop_location(),
                base, arguments,
            }))
        }
    }

    fn parse_new_expression_start(&mut self) -> Rc<Expression> {
        if self.peek(Token::New) {
            let start = self.token_location();
            self.next();
            self.parse_new_expression(start)
        } else if self.peek(Token::Super) {
            self.parse_super_expression_followed_by_property_operator()
        } else {
            self.parse_primary_expression()
        }
    }

    fn parse_super_expression_followed_by_property_operator(&mut self) -> Rc<Expression> {
        self.mark_location();
        self.duplicate_location();
        self.next();
        let arguments = if self.peek(Token::ParenOpen) { Some(self.parse_arguments()) } else { None };
        let super_expr = Rc::new(Expression::Super(SuperExpression {
            location: self.pop_location(),
            object: arguments,
        }));

        if self.consume(Token::SquareOpen) {
            let key = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
            self.non_greedy_expect(Token::SquareClose);
            Rc::new(Expression::ComputedMember(ComputedMemberExpression {
                location: self.pop_location(),
                base: super_expr, asdoc: None, key,
            }))
        } else {
            self.non_greedy_expect(Token::Dot);
            let identifier = self.parse_qualified_identifier();
            Rc::new(Expression::Member(MemberExpression {
                location: self.pop_location(),
                base: super_expr, identifier,
            }))
        }
    }

    fn parse_arguments(&mut self) -> Vec<Rc<Expression>> {
        self.non_greedy_expect(Token::ParenOpen);
        let mut arguments = vec![];
        if !self.peek(Token::ParenClose) {
            arguments.push(self.parse_expression(ParserExpressionContext {
                allow_in: true,
                min_precedence: OperatorPrecedence::AssignmentAndOther,
                ..default()
            }));
            while self.consume(Token::Comma) {
                arguments.push(self.parse_expression(ParserExpressionContext {
                    allow_in: true,
                    min_precedence: OperatorPrecedence::AssignmentAndOther,
                    ..default()
                }));
            }
        }
        self.non_greedy_expect(Token::ParenClose);
        arguments
    }

    fn parse_new_subexpression(&mut self) -> Rc<Expression> {
        let mut base = self.parse_new_expression_start();
        loop {
            if self.consume(Token::SquareOpen) {
                self.push_location(&base.location());
                let key = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
                self.non_greedy_expect(Token::SquareClose);
                base = Rc::new(Expression::ComputedMember(ComputedMemberExpression {
                    location: self.pop_location(),
                    base, asdoc: None, key,
                }));
            } else if self.consume(Token::Dot) {
                self.push_location(&base.location());
                if self.consume(Token::Lt) {
                    let mut arguments = vec![];
                    arguments.push(self.parse_type_expression());
                    while self.consume(Token::Comma) {
                        arguments.push(self.parse_type_expression());
                    }
                    self.non_greedy_expect_type_parameters_gt();
                    base = Rc::new(Expression::WithTypeArguments(ApplyTypeExpression {
                        location: self.pop_location(),
                        base, arguments
                    }));
                } else {
                    let identifier = self.parse_qualified_identifier();
                    base = Rc::new(Expression::Member(MemberExpression {
                        location: self.pop_location(),
                        base, identifier,
                    }));
                }
            } else {
                break;
            }
        }
        base
    }

    fn parse_primary_expression(&mut self) -> Rc<Expression> {
        if let Token::Identifier(id) = self.token.0.clone() {
            let id_location = self.token_location();
            self.next();

            /*
            // EmbedExpression
            if self.peek(Token::BlockOpen) && id == "embed" && self.previous_token.1.character_count() == "embed".len() {
                return self.finish_embed_expression(id_location);
            }
            */

            let id = Rc::new(Expression::QualifiedIdentifier(QualifiedIdentifier {
                location: id_location.clone(),
                attribute: false,
                qualifier: None,
                id: QualifiedIdentifierIdentifier::Id((id, id_location.clone())),
            }));
            if self.peek(Token::ColonColon) {
                self.push_location(&id_location.clone());
                let ql = self.pop_location();
                let id = self.finish_qualified_identifier(false, ql, id);
                Rc::new(Expression::QualifiedIdentifier(id))
            } else {
                id
            }
        } else if self.peek(Token::Null) {
            self.mark_location();
            self.next();
            Rc::new(Expression::NullLiteral(NullLiteral {
                location: self.pop_location(),
            }))
        } else if self.peek(Token::False) {
            self.mark_location();
            self.next();
            Rc::new(Expression::BooleanLiteral(BooleanLiteral {
                location: self.pop_location(),
                value: false,
            }))
        } else if self.peek(Token::True) {
            self.mark_location();
            self.next();
            Rc::new(Expression::BooleanLiteral(BooleanLiteral {
                location: self.pop_location(),
                value: true,
            }))
        } else if let Token::Number(n, suffix) = self.token.0.clone() {
            self.mark_location();
            self.next();
            Rc::new(Expression::NumericLiteral(NumericLiteral {
                location: self.pop_location(),
                value: n,
                suffix,
            }))
        } else if let Token::String(ref s) = self.token.0.clone() {
            self.mark_location();
            self.next();
            Rc::new(Expression::StringLiteral(StringLiteral {
                location: self.pop_location(),
                value: s.clone(),
            }))
        } else if self.peek(Token::This) {
            self.mark_location();
            self.next();
            Rc::new(Expression::ThisLiteral(ThisLiteral {
                location: self.pop_location(),
            }))
        } else if self.peek(Token::Div) || self.peek(Token::DivideAssign) {
            self.mark_location();
            self.token = self.tokenizer.scan_regexp_literal(self.token.1.clone(), if self.peek(Token::DivideAssign) { "=".into() } else { "".into() });
            let Token::RegExp { ref body, ref flags } = self.token.0.clone() else {
                panic!();
            };
            self.next();
            Rc::new(Expression::RegExpLiteral(RegExpLiteral {
                location: self.pop_location(),
                body: body.clone(), flags: flags.clone(),
            }))
        // `@`
        } else if self.peek(Token::Attribute) {
            self.mark_location();
            let id = self.parse_qualified_identifier();
            Rc::new(Expression::QualifiedIdentifier(id))
        // Parentheses
        } else if self.peek(Token::ParenOpen) {
            return self.parse_paren_list_expr_or_qual_id();
        // XMLList, XMLElement, XMLMarkup
        } else if self.peek(Token::Lt) {
            if let Some(token) = self.tokenizer.scan_xml_markup(self.token_location()) {
                self.token = token;
            }
            let start = self.token_location();
            if let Token::XmlMarkup(content) = &self.token.0.clone() {
                self.mark_location();
                self.next();
                Rc::new(Expression::XmlMarkup(XmlMarkupExpression {
                    location: self.pop_location(),
                    markup: content.clone(),
                }))
            } else {
                self.parse_xml_element_or_xml_list(start)
            }
        // ArrayInitializer
        } else if self.peek(Token::SquareOpen) {
            self.parse_array_initializer()
        } else if self.peek(Token::BlockOpen) {
            self.parse_object_initializer()
        // QualifiedIdentifier
        } else if
                self.peek(Token::Times)
            ||  self.peek(Token::Public) || self.peek(Token::Private)
            ||  self.peek(Token::Protected) || self.peek(Token::Internal) {
            let id = self.parse_qualified_identifier();
            Rc::new(Expression::QualifiedIdentifier(id))
        } else {
            self.add_syntax_error(&self.token_location(), DiagnosticKind::ExpectingExpression, diagarg![self.token.0.clone()]);
            self.create_invalidated_expression(&self.tokenizer.cursor_location())
        }
    }

    /*
    fn finish_embed_expression(&mut self, start: Location) -> Rc<Expression> {
        self.push_location(&start);
        let descriptor = self.parse_object_initializer().clone();
        let Expression::ObjectInitializer(descriptor) = descriptor.as_ref() else {
            panic!();
        };
        return Rc::new(Expression::Embed(EmbedExpression {
            location: self.pop_location(),
            description: descriptor.clone(),
        }));
    }
    */

    fn parse_array_initializer(&mut self) -> Rc<Expression> {
        self.mark_location();

        let asdoc = self.parse_asdoc();

        self.non_greedy_expect(Token::SquareOpen);

        let mut elements: Vec<Element> = vec![];

        while !self.peek(Token::SquareClose) {
            let mut ellipses = false;
            while self.consume(Token::Comma) {
                elements.push(Element::Elision);
                ellipses = true;
            }
            if !ellipses  {
                if self.peek(Token::Ellipsis) {
                    self.mark_location();
                    self.next();
                    elements.push(Element::Rest((self.parse_expression(ParserExpressionContext {
                        allow_in: true,
                        min_precedence: OperatorPrecedence::AssignmentAndOther,
                        ..default()
                    }), self.pop_location())));
                } else {
                    elements.push(Element::Expression(self.parse_expression(ParserExpressionContext {
                        allow_in: true,
                        min_precedence: OperatorPrecedence::AssignmentAndOther,
                        ..default()
                    })));
                }
            }
            if !self.consume(Token::Comma) {
                break;
            }
        }
        self.non_greedy_expect(Token::SquareClose);
        Rc::new(Expression::ArrayLiteral(ArrayLiteral {
            location: self.pop_location(),
            asdoc,
            elements,
        }))
    }

    fn parse_xml_element_or_xml_list(&mut self, start: Location) -> Rc<Expression> {
        self.next_ie_xml_tag();
        if self.consume_and_ie_xml_content(Token::Gt) {
            self.push_location(&start);
            let content = self.parse_xml_content();
            self.non_greedy_expect_and_ie_xml_tag(Token::XmlLtSlash);
            self.non_greedy_expect(Token::Gt);
            return Rc::new(Expression::XmlList(XmlListExpression {
                location: self.pop_location(),
                content,
            }));
        }

        self.push_location(&start);
        let element = Rc::new(self.parse_xml_element(start, true));
        return Rc::new(Expression::Xml(XmlExpression {
            location: self.pop_location(),
            element,
        }));
    }

    /// Parses XMLElement starting from its XMLTagContent.
    fn parse_xml_element(&mut self, start: Location, ends_at_ie_div: bool) -> XmlElement {
        self.push_location(&start);
        let name = self.parse_xml_tag_name();
        let mut attributes: Vec<Rc<XmlAttribute>> = vec![];
        let mut attribute_expression: Option<Rc<Expression>> = None;
        while self.consume_and_ie_xml_tag(Token::XmlWhitespace) {
            if self.consume(Token::BlockOpen) {
                let expr = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::AssignmentAndOther, ..default() });
                self.expect_and_ie_xml_tag(Token::BlockClose);
                attribute_expression = Some(expr);
                self.consume_and_ie_xml_tag(Token::XmlWhitespace);
                break;
            } else if matches!(self.token.0, Token::XmlName(_)) {
                self.mark_location();
                let name = self.parse_xml_name();
                self.consume_and_ie_xml_tag(Token::XmlWhitespace);
                self.non_greedy_expect_and_ie_xml_tag(Token::Assign);
                let mut value = XmlAttributeValue::Value(("".into(), self.token.1.clone()));
                if !self.expecting_token_error {
                    self.consume_and_ie_xml_tag(Token::XmlWhitespace);
                    if self.consume(Token::BlockOpen) {
                        let expr = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::AssignmentAndOther, ..default() });
                        self.expect_and_ie_xml_tag(Token::BlockClose);
                        value = XmlAttributeValue::Expression(expr);
                    } else {
                        value = XmlAttributeValue::Value(self.parse_xml_attribute_value());
                    }
                }
                attributes.push(Rc::new(XmlAttribute {
                    location: self.pop_location(),
                    name, value
                }));
            } else {
                break;
            }
        }

        let mut content: Option<Vec<Rc<XmlContent>>> = None;
        let mut closing_name: Option<XmlTagName> = None;

        let is_empty;

        if ends_at_ie_div {
            is_empty = self.consume(Token::XmlSlashGt);
        } else {
            is_empty = self.consume_and_ie_xml_content(Token::XmlSlashGt);
        }

        if !is_empty {
            self.expect_and_ie_xml_content(Token::Gt);
            content = Some(self.parse_xml_content());
            self.non_greedy_expect_and_ie_xml_tag(Token::XmlLtSlash);
            closing_name = Some(self.parse_xml_tag_name());
            self.consume_and_ie_xml_tag(Token::XmlWhitespace);
            if ends_at_ie_div {
                self.non_greedy_expect(Token::Gt);
            } else {
                self.non_greedy_expect_and_ie_xml_content(Token::Gt);
            }
        }

        XmlElement {
            location: self.pop_location(),
            name,
            attributes,
            attribute_expression,
            content,
            closing_name,
        }
    }
    
    fn parse_xml_attribute_value(&mut self) -> (String, Location) {
        if let Token::XmlAttributeValue(value) = self.token.0.clone() {
            let location = self.token_location();
            self.next_ie_xml_tag();
            return (value, location);
        } else {
            self.add_syntax_error(&self.token_location(), DiagnosticKind::ExpectingXmlAttributeValue, diagarg![self.token.0.clone()]);
            ("".into(), self.tokenizer.cursor_location())
        }
    }

    fn parse_xml_tag_name(&mut self) -> XmlTagName {
        if self.consume(Token::BlockOpen) {
            let expr = self.parse_expression(ParserExpressionContext {
                allow_in: true,
                min_precedence: OperatorPrecedence::AssignmentAndOther,
                ..default()
            });
            self.expect_and_ie_xml_tag(Token::BlockClose);
            XmlTagName::Expression(expr)
        } else {
            XmlTagName::Name(self.parse_xml_name())
        }
    }

    fn parse_xml_name(&mut self) -> (String, Location) {
        if let Token::XmlName(name) = self.token.0.clone() {
            let name_location = self.token_location();
            self.next_ie_xml_tag();
            return (name, name_location);
        } else {
            self.add_syntax_error(&self.token_location(), DiagnosticKind::ExpectingXmlName, diagarg![self.token.0.clone()]);
            (INVALIDATED_IDENTIFIER.into(), self.tokenizer.cursor_location())
        }
    }

    /// Parses XMLContent until a `</` token.
    fn parse_xml_content(&mut self) -> Vec<Rc<XmlContent>> {
        let mut content = vec![];
        while !self.peek(Token::XmlLtSlash) {
            if self.consume(Token::BlockOpen) {
                let expr = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::AssignmentAndOther, ..default() });
                self.expect_and_ie_xml_content(Token::BlockClose);
                content.push(Rc::new(XmlContent::Expression(expr)));
            } else if let Token::XmlMarkup(markup) = self.token.0.clone() {
                let location = self.token_location();
                self.next_ie_xml_content();
                content.push(Rc::new(XmlContent::Markup((markup, location))));
            } else if let Token::XmlText(text) = self.token.0.clone() {
                if self.tokenizer.characters().reached_end() {
                    self.expect_and_ie_xml_content(Token::XmlLtSlash);
                    break;
                }
                let location = self.token_location();
                self.next_ie_xml_content();
                content.push(Rc::new(XmlContent::Characters((text, location))));
            } else if self.consume_and_ie_xml_tag(Token::Lt) {
                let start = self.token_location();
                let element = self.parse_xml_element(start, false);
                content.push(Rc::new(XmlContent::Element(Rc::new(element))));
            } else if self.peek(Token::Eof) {
                break;
            } else {
                self.expect_and_ie_xml_content(Token::XmlLtSlash);
            }
        }
        content
    }

    fn finish_paren_list_expr_or_qual_id(&mut self, start: Location, left: Rc<Expression>) -> Rc<Expression> {
        if self.peek(Token::ColonColon) && !matches!(left.as_ref(), Expression::Sequence(_)) {
            self.push_location(&start);
            let ql = self.pop_location();
            let left = Rc::new(Expression::Paren(ParenExpression {
                location: ql.clone(),
                expression: left,
            }));
            let id = self.finish_qualified_identifier(false, ql, left);
            return Rc::new(Expression::QualifiedIdentifier(id));
        }
        self.push_location(&start);
        return Rc::new(Expression::Paren(ParenExpression {
            location: self.pop_location(),
            expression: left,
        }));
    }

    /// Parses either a ParenListExpression, (), or a QualifiedIdentifier
    fn parse_paren_list_expr_or_qual_id(&mut self) -> Rc<Expression> {
        let start = self.token_location();
        self.non_greedy_expect(Token::ParenOpen);

        let expr = self.parse_expression(ParserExpressionContext {
            min_precedence: OperatorPrecedence::List,
            allow_in: true,
            ..default()
        });

        self.non_greedy_expect(Token::ParenClose);
        self.finish_paren_list_expr_or_qual_id(start, expr)
    }

    fn parse_opt_reserved_namespace(&mut self) -> Option<Rc<Expression>> {
        let loc = self.token.1.clone();
        if self.consume(Token::Public) {
            Some(Rc::new(Expression::ReservedNamespace(ReservedNamespaceExpression::Public(loc))))
        } else if self.consume(Token::Private) {
            Some(Rc::new(Expression::ReservedNamespace(ReservedNamespaceExpression::Private(loc))))
        } else if self.consume(Token::Protected) {
            Some(Rc::new(Expression::ReservedNamespace(ReservedNamespaceExpression::Protected(loc))))
        } else if self.consume(Token::Internal) {
            Some(Rc::new(Expression::ReservedNamespace(ReservedNamespaceExpression::Internal(loc))))
        } else {
            None
        }
    }

    fn parse_qualified_identifier(&mut self) -> QualifiedIdentifier {
        self.mark_location();

        let attribute = self.consume(Token::Attribute);
        if attribute && self.peek(Token::SquareOpen) {
            let brackets = self.parse_brackets();
            return QualifiedIdentifier {
                location: self.pop_location(),
                attribute,
                qualifier: None,
                id: QualifiedIdentifierIdentifier::Brackets(brackets),
            };
        }

        // public, private, protected, internal
        if let Some(qual) = self.parse_opt_reserved_namespace() {
            if self.peek(Token::ColonColon) {
                let ql = self.pop_location();
                return self.finish_qualified_identifier(attribute, ql, qual);
            } else {
                let id = QualifiedIdentifier {
                    location: self.pop_location(),
                    attribute,
                    qualifier: None,
                    id: QualifiedIdentifierIdentifier::Id((qual.to_reserved_namespace_string().unwrap(), qual.location())),
                };
                return id;
            }
        }

        let mut id: Option<String> = None;

        // IdentifierName
        if let Token::Identifier(id_1) = self.token.0.clone() {
            id = Some(id_1);
        } else {
            if let Some(id_1) = self.token.0.reserved_word_name() {
                id = Some(id_1);
            } else if self.peek(Token::Times) {
                id = Some("*".to_owned());
            }
        }

        if let Some(id) = id {
            let id_location = self.token_location();
            self.next();
            if self.peek(Token::ColonColon) {
                let id = QualifiedIdentifier {
                    location: id_location.clone(),
                    attribute: false,
                    qualifier: None,
                    id: QualifiedIdentifierIdentifier::Id((id, id_location.clone())),
                };
                let id = Rc::new(Expression::QualifiedIdentifier(id));
                let ql = self.pop_location();
                return self.finish_qualified_identifier(attribute, ql, id);
            } else {
                let id = QualifiedIdentifier {
                    location: self.pop_location(),
                    attribute,
                    qualifier: None,
                    id: QualifiedIdentifierIdentifier::Id((id, id_location.clone())),
                };
                return id;
            }
        }

        // (q)::x
        if self.peek(Token::ParenOpen) {
            let qual = self.parse_paren_expression();
            let ql = self.pop_location();
            let qual = Rc::new(Expression::Paren(ParenExpression {
                location: ql.clone(),
                expression: qual,
            }));
            return self.finish_qualified_identifier(attribute, ql, qual);
        }

        self.add_syntax_error(&self.token_location(), DiagnosticKind::ExpectingIdentifier, diagarg![self.token.0.clone()]);
        QualifiedIdentifier {
            location: self.pop_location(),
            attribute: false,
            qualifier: None,
            id: QualifiedIdentifierIdentifier::Id(("".into(), self.tokenizer.cursor_location())),
        }
    }

    fn parse_non_attribute_qualified_identifier(&mut self) -> QualifiedIdentifier {
        self.mark_location();

        let attribute = false;

        // public, private, protected, internal
        if let Some(qual) = self.parse_opt_reserved_namespace() {
            if self.peek(Token::ColonColon) {
                let ql = self.pop_location();
                return self.finish_qualified_identifier(attribute, ql, qual);
            } else {
                let id = QualifiedIdentifier {
                    location: self.pop_location(),
                    attribute,
                    qualifier: None,
                    id: QualifiedIdentifierIdentifier::Id((qual.to_reserved_namespace_string().unwrap(), qual.location())),
                };
                return id;
            }
        }

        let mut id: Option<String> = None;

        // IdentifierName
        if let Token::Identifier(id_1) = self.token.0.clone() {
            id = Some(id_1);
        } else {
            if let Some(id_1) = self.token.0.reserved_word_name() {
                id = Some(id_1);
            } else if self.peek(Token::Times) {
                id = Some("*".to_owned());
            }
        }

        if let Some(id) = id {
            let id_location = self.token_location();
            self.next();
            if self.peek(Token::ColonColon) {
                let id = QualifiedIdentifier {
                    location: id_location.clone(),
                    attribute: false,
                    qualifier: None,
                    id: QualifiedIdentifierIdentifier::Id((id, id_location.clone())),
                };
                let id = Rc::new(Expression::QualifiedIdentifier(id));
                let ql = self.pop_location();
                return self.finish_qualified_identifier(attribute, ql, id);
            } else {
                let id = QualifiedIdentifier {
                    location: self.pop_location(),
                    attribute,
                    qualifier: None,
                    id: QualifiedIdentifierIdentifier::Id((id, id_location.clone())),
                };
                return id;
            }
        }

        // (q)::x
        if self.peek(Token::ParenOpen) {
            let qual = self.parse_paren_expression();
            let ql = self.pop_location();
            let qual = Rc::new(Expression::Paren(ParenExpression {
                location: ql.clone(),
                expression: qual,
            }));
            return self.finish_qualified_identifier(attribute, ql, qual);
        }

        self.add_syntax_error(&self.token_location(), DiagnosticKind::ExpectingIdentifier, diagarg![self.token.0.clone()]);
        QualifiedIdentifier {
            location: self.pop_location(),
            attribute: false,
            qualifier: None,
            id: QualifiedIdentifierIdentifier::Id(("".into(), self.tokenizer.cursor_location())),
        }
    }

    /// Expects a colon-colon and finishes a qualified identifier.
    fn finish_qualified_identifier(&mut self, attribute: bool, start_location: Location, qual: Rc<Expression>) -> QualifiedIdentifier {
        self.push_location(&start_location);
        self.non_greedy_expect(Token::ColonColon);

        // `::` may be followed by one of { IdentifierName, `*`, Brackets }

        // IdentifierName
        if let Some(id) = self.consume_identifier(true) {
            QualifiedIdentifier {
                location: self.pop_location(),
                attribute,
                qualifier: Some(qual),
                id: QualifiedIdentifierIdentifier::Id(id),
            }
        // `*`
        } else if self.peek(Token::Times) {
            let id_location = self.token_location();
            self.next();
            QualifiedIdentifier {
                location: self.pop_location(),
                attribute,
                qualifier: Some(qual),
                id: QualifiedIdentifierIdentifier::Id(("*".into(), id_location)),
            }
        // Brackets
        } else if self.peek(Token::SquareOpen) {
            let brackets = self.parse_brackets();
            QualifiedIdentifier {
                location: self.pop_location(),
                attribute,
                qualifier: Some(qual),
                id: QualifiedIdentifierIdentifier::Brackets(brackets),
            }
        } else {
            self.add_syntax_error(&self.token_location(), DiagnosticKind::ExpectingIdentifier, diagarg![self.token.0.clone()]);
            QualifiedIdentifier {
                location: self.pop_location(),
                attribute,
                qualifier: Some(qual),
                id: QualifiedIdentifierIdentifier::Id(("".into(), self.tokenizer.cursor_location())),
            }
        }
    }

    fn parse_brackets(&mut self) -> Rc<Expression> {
        self.non_greedy_expect(Token::SquareOpen);
        let expr = self.parse_expression(ParserExpressionContext {
            min_precedence: OperatorPrecedence::List,
            allow_in: true,
            ..default()
        });
        self.non_greedy_expect(Token::SquareClose);
        expr
    }

    fn parse_paren_expression(&mut self) -> Rc<Expression> {
        self.non_greedy_expect(Token::ParenOpen);
        let expr = self.parse_expression(ParserExpressionContext {
            min_precedence: OperatorPrecedence::AssignmentAndOther,
            allow_in: true,
            ..default()
        });
        self.non_greedy_expect(Token::ParenClose);
        expr
    }

    fn parse_paren_list_expression(&mut self) -> Rc<Expression> {
        self.non_greedy_expect(Token::ParenOpen);
        let expr = self.parse_expression(ParserExpressionContext {
            min_precedence: OperatorPrecedence::List,
            allow_in: true,
            ..default()
        });
        self.non_greedy_expect(Token::ParenClose);
        expr
    }

    fn parse_typed_destructuring(&mut self) -> TypedDestructuring {
        self.mark_location();
        let mut destructuring: Rc<Expression>;
        if self.peek(Token::BlockOpen) {
            destructuring = self.parse_object_initializer();
        } else if self.peek(Token::SquareOpen) {
            destructuring = self.parse_array_initializer();
        } else {
            let id = self.expect_identifier(true);
            let id = QualifiedIdentifier {
                location: id.1.clone(),
                attribute: false,
                qualifier: None,
                id: QualifiedIdentifierIdentifier::Id(id.clone()),
            };
            destructuring = Rc::new(Expression::QualifiedIdentifier(id));
        }
        if self.consume(Token::Exclamation) {
            self.push_location(&destructuring.location());
            destructuring = Rc::new(Expression::Unary(UnaryExpression {
                location: self.pop_location(),
                operator: Operator::NonNull,
                expression: destructuring.clone(),
            }));
        }
        if !destructuring.is_valid_destructuring() {
            self.add_syntax_error(&destructuring.location(), DiagnosticKind::MalformedDestructuring, vec![])
        }
        let type_annotation = if self.consume(Token::Colon) { Some(self.parse_type_expression()) } else { None };
        TypedDestructuring {
            location: self.pop_location(),
            destructuring,
            type_annotation,
        }
    }

    pub fn parse_type_expression(&mut self) -> Rc<Expression> {
        let start = self.token_location();
        let (mut base, wrap_nullable) = self.parse_type_expression_start();

        loop {
            if self.consume(Token::Dot) {
                base = self.parse_dot_subexpression(base);
            } else if self.consume(Token::Question) {
                self.push_location(&base.location());
                base = Rc::new(Expression::NullableType(NullableTypeExpression {
                    location: self.pop_location(),
                    base,
                }));
            } else if self.consume(Token::Exclamation) {
                self.push_location(&base.location());
                base = Rc::new(Expression::NonNullableType(NonNullableTypeExpression {
                    location: self.pop_location(),
                    base,
                }));
            } else {
                break;
            }
        }
        
        if wrap_nullable {
            self.push_location(&start);
            base = Rc::new(Expression::NullableType(NullableTypeExpression {
                location: self.pop_location(),
                base,
            }));
        }

        base
    }

    fn parse_type_expression_start(&mut self) -> (Rc<Expression>, bool) {
        // Allow a `?` prefix to wrap a type into nullable.
        let wrap_nullable = self.consume(Token::Question);

        // Parenthesized
        if self.peek(Token::ParenOpen) {
            self.mark_location();
            let expression = self.parse_type_expression();
            (Rc::new(Expression::Paren(ParenExpression {
                location: self.pop_location(),
                expression,
            })), wrap_nullable)
        }
        // `function`
        else if self.peek(Token::Function) {
            (self.parse_function_type_expression(), wrap_nullable)
        // `void`
        } else if self.peek(Token::Void) {
            self.mark_location();
            self.next();
            (Rc::new(Expression::VoidType(VoidTypeExpression {
                location: self.pop_location(),
            })), wrap_nullable)
        // [T]
        // [T1, T2, ...Tn]
        } else if self.peek(Token::SquareOpen) {
            let mut elements = vec![];
            self.mark_location();
            self.next();
            elements.push(self.parse_type_expression());
            if self.consume(Token::SquareClose) {
                (Rc::new(Expression::ArrayType(ArrayTypeExpression {
                    location: self.pop_location(),
                    expression: elements[0].clone(),
                })), wrap_nullable)
            } else {
                self.non_greedy_expect(Token::Comma);
                elements.push(self.parse_type_expression());
                while self.consume(Token::Comma) {
                    if self.peek(Token::SquareClose) {
                        break;
                    }
                    elements.push(self.parse_type_expression());
                }
                self.non_greedy_expect(Token::SquareClose);
                (Rc::new(Expression::TupleType(TupleTypeExpression {
                    location: self.pop_location(),
                    expressions: elements,
                })), wrap_nullable)
            }
        } else if self.peek(Token::Times) {
            let location = self.token_location();
            self.next();
            (Rc::new(Expression::AnyType(AnyTypeExpression {
                location,
            })), wrap_nullable)
        // Identifier
        } else {
            let id = self.parse_qualified_identifier();
            (Rc::new(Expression::QualifiedIdentifier(id)), wrap_nullable)
        }
    }

    fn parse_function_type_expression(&mut self) -> Rc<Expression> {
        self.mark_location();
        self.next();

        let mut parameters = vec![];
        self.non_greedy_expect(Token::ParenOpen);
        if !self.expecting_token_error {
            if !self.peek(Token::ParenClose) {
                parameters.push(self.parse_function_type_parameter());
                while self.consume(Token::Comma) {
                    parameters.push(self.parse_function_type_parameter());
                }
            }
            self.non_greedy_expect(Token::ParenClose);
            self.validate_parameter_list(parameters.iter().map(|p| (p.kind, p.location.clone())).collect::<Vec<_>>());
        }

        let mut result_type = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        self.non_greedy_expect(Token::Colon);
        if !self.expecting_token_error {
            result_type = self.parse_type_expression();
        }
        Rc::new(Expression::FunctionType(FunctionTypeExpression {
            location: self.pop_location(),
            parameters,
            result_type: Some(result_type),
        }))
    }

    fn parse_function_type_parameter(&mut self) -> Rc<FunctionTypeParameter> {
        self.mark_location();
        let rest = self.consume(Token::Ellipsis);
        let type_expression: Option<Rc<Expression>> = if rest && self.peek(Token::ParenClose) {
            None
        } else {
            Some(self.parse_type_expression())
        };
        let optional = !rest && self.consume(Token::Assign);
        let location = self.pop_location();
        Rc::new(FunctionTypeParameter {
            location,
            type_expression,
            kind: if rest {
                ParameterKind::Rest
            } else if optional {
                ParameterKind::Optional
            } else {
                ParameterKind::Required
            },
        })
    }

    fn parse_variable_binding(&mut self, allow_in: bool) -> VariableBinding {
        let destructuring = self.parse_typed_destructuring();
        let initializer = if self.consume(Token::Assign) {
            Some(self.parse_expression(ParserExpressionContext {
                allow_in,
                min_precedence: OperatorPrecedence::AssignmentAndOther,
                ..default()
            }))
        } else {
            None
        };
        VariableBinding {
            destructuring,
            initializer,
        }
    }

    fn parse_semicolon(&mut self) -> bool {
        self.consume(Token::Semicolon) || self.peek(Token::BlockClose) || self.previous_token.1.line_break(&self.token.1)
    }

    fn parse_substatement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        self.parse_statement(context)
    }

    fn parse_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        // ExpressionStatement or LabeledStatement
        if let Token::Identifier(id) = &self.token.0.clone() {
            let id = (id.clone(), self.token_location());
            self.next();
            self.parse_statement_starting_with_identifier(context, id)
        // SuperStatement or ExpressionStatement with `super`
        } else if self.peek(Token::Super) {
            self.mark_location();
            self.next();
            let arguments = if self.peek(Token::ParenOpen) { Some(self.parse_arguments()) } else { None };
            let mut semicolon = false;
            if arguments.is_some() {
                semicolon = self.parse_semicolon();
            }
            if !semicolon && (self.peek(Token::Dot) || self.peek(Token::SquareOpen)) {
                if !(self.peek(Token::Dot) || self.peek(Token::SquareOpen)) {
                    self.non_greedy_expect(Token::Dot);
                }
                self.duplicate_location();
                // ExpressionStatement (`super`...)
                let mut expr = Rc::new(Expression::Super(SuperExpression {
                    location: self.pop_location(),
                    object: arguments,
                }));
                expr = self.parse_subexpressions(expr, ParserExpressionContext {
                    allow_in: true,
                    min_precedence: OperatorPrecedence::List,
                    ..default()
                });
                let semicolon = self.parse_semicolon();
                (Rc::new(Directive::ExpressionStatement(ExpressionStatement {
                    location: self.pop_location(),
                    expression: expr,
                })), semicolon)
            } else {
                // SuperStatement
                let node = Rc::new(Directive::SuperStatement(SuperStatement {
                    location: self.pop_location(),
                    arguments: arguments.unwrap(),
                }));

                // Check whether super statement is allowed here
                let allowed_here;
                if context.may_contain_super_statement() {
                    allowed_here = !context.super_statement_found();
                    context.set_super_statement_found(true);
                } else {
                    allowed_here = false;
                }

                if !allowed_here {
                    self.add_syntax_error(&node.location(), DiagnosticKind::NotAllowedHere, diagarg![Token::Super]);
                }

                (node, semicolon)
            }
        // EmptyStatement
        } else if self.peek(Token::Semicolon) {
            self.mark_location();
            self.next();
            (Rc::new(Directive::EmptyStatement(EmptyStatement {
                location: self.pop_location(),
            })), true)
        // Block
        } else if self.peek(Token::BlockOpen) {
            let context = if context.is_top_level_or_package() || context.is_type_block() {
                context.clone()
            } else {
                context.override_control_context(true, ParserControlFlowContext {
                    breakable: true,
                    iteration: false,
                })
            };
            let block = self.parse_block(context);
            (Rc::new(Directive::Block(block)), true)
        // IfStatement
        } else if self.peek(Token::If) {
            self.parse_if_statement(context)
        // SwitchStatement
        // `switch type`
        } else if self.peek(Token::Switch) {
            self.parse_switch_statement(context)
        // DoStatement
        } else if self.peek(Token::Do) {
            self.parse_do_statement(context)
        // WhileStatement
        } else if self.peek(Token::While) {
            self.parse_while_statement(context)
        // ForStatement
        // `for..in`
        // `for each`
        } else if self.peek(Token::For) {
            self.parse_for_statement(context)
        // WithStatement
        } else if self.peek(Token::With) {
            self.parse_with_statement(context)
        // BreakStatement
        } else if self.peek(Token::Break) {
            self.parse_break_statement(context)
        // ContinueStatement
        } else if self.peek(Token::Continue) {
            self.parse_continue_statement(context)
        // ReturnStatement
        } else if self.peek(Token::Return) {
            self.parse_return_statement(context)
        // ThrowStatement
        } else if self.peek(Token::Throw) {
            self.parse_throw_statement(context)
        // TryStatement
        } else if self.peek(Token::Try) {
            self.parse_try_statement(context)
        // `default xml namespace = expression`
        } else if self.peek(Token::Default) {
            self.parse_default_xml_namespace_statement()
        // ExpressionStatement
        } else {
            self.mark_location();

            // Store offset for patching error
            let i = self.tokenizer.characters().index();

            let exp = self.parse_expression(ParserExpressionContext {
                allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
            });

            // Patch error
            if i == self.tokenizer.characters().index() {
                self.patch_syntax_error(DiagnosticKind::ExpectingExpression, DiagnosticKind::ExpectingStatement, diagarg![self.token.0.clone()]);
            }

            let semicolon = if exp.is_invalidated() {
                self.next();
                true
            } else { self.parse_semicolon() };
            (Rc::new(Directive::ExpressionStatement(ExpressionStatement {
                location: self.pop_location(),
                expression: exp,
            })), semicolon)
        }
    }

    fn parse_statement_starting_with_identifier(&mut self, context: ParserDirectiveContext, id: (String, Location)) -> (Rc<Directive>, bool) {
        self.push_location(&id.1);
        let id_location = id.1.clone();

        // LabeledStatement
        if self.consume(Token::Colon) {
            let (substatement, semicolon) = self.parse_substatement(context.put_label(id.0.clone()));
            let labeled = Rc::new(Directive::LabeledStatement(LabeledStatement {
                location: self.pop_location(),
                label: id.clone(),
                substatement,
            }));
            return (labeled, semicolon);
        }

        let mut exp: Rc<Expression>;

        /*
        // EmbedExpression
        if self.peek(Token::BlockOpen) && id.0 == "embed" && self.previous_token.1.character_count() == "embed".len() {
            exp = self.finish_embed_expression(id_location);
        } else {
        */
        {
            let id = Rc::new(Expression::QualifiedIdentifier(QualifiedIdentifier {
                location: id_location.clone(),
                attribute: false,
                qualifier: None,
                id: QualifiedIdentifierIdentifier::Id(id.clone()),
            }));
            if self.peek(Token::ColonColon) {
                self.push_location(&id_location.clone());
                let ql = self.pop_location();
                let id = self.finish_qualified_identifier(false, ql, id);
                exp = Rc::new(Expression::QualifiedIdentifier(id));
            } else {
                exp = id;
            }
        }

        exp = self.parse_subexpressions(exp, ParserExpressionContext {
            allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
        });
        let semicolon = self.parse_semicolon();
        (Rc::new(Directive::ExpressionStatement(ExpressionStatement {
            location: self.pop_location(),
            expression: exp,
        })), semicolon)
    }

    fn parse_qualified_identifier_statement_or_config(&mut self, context: ParserDirectiveContext, id: (String, Location), asdoc: Option<Rc<Asdoc>>) -> (Rc<Directive>, bool) {
        self.push_location(&id.1);
        let id_location = id.1.clone();
        let id = Rc::new(Expression::QualifiedIdentifier(QualifiedIdentifier {
            location: id_location.clone(),
            attribute: false,
            qualifier: None,
            id: QualifiedIdentifierIdentifier::Id(id.clone()),
        }));
        self.push_location(&id_location.clone());
        let ql = self.pop_location();
        let id = self.finish_qualified_identifier(false, ql, id);
        let mut exp = Rc::new(Expression::QualifiedIdentifier(id));
        exp = self.parse_subexpressions(exp, ParserExpressionContext {
            allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
        });

        // Parse CONFIG::VAR_NAME
        if let Some(result) = self.parse_opt_config(&exp, asdoc.clone(), context.clone()) {
            return result;
        }

        let semicolon = self.parse_semicolon();
        (Rc::new(Directive::ExpressionStatement(ExpressionStatement {
            location: self.pop_location(),
            expression: exp,
        })), semicolon)
    }

    fn parse_opt_config(&mut self, exp: &Rc<Expression>, asdoc: Option<Rc<Asdoc>>, context: ParserDirectiveContext) -> Option<(Rc<Directive>, bool)> {
        if self.peek_annotatable_directive_identifier_name() {
            match exp.to_configuration_identifier(self) {
                Ok(Some((q, constant_name, metadata))) => {
                    self.push_location(&exp.location());
                    let mut context = AnnotatableContext {
                        start_location: exp.location(),
                        asdoc: self.parse_asdoc().or(asdoc),
                        attributes: metadata,
                        context,
                        directive_context_keyword: None,
                    };
                    self.parse_attribute_keywords_or_expressions(&mut context);
                    let (directive, semicolon) = self.parse_annotatable_directive(context);
                    return Some((Rc::new(Directive::ConfigurationDirective(ConfigurationDirective {
                        location: self.pop_location(),
                        namespace: q,
                        constant_name,
                        directive,
                    })), semicolon));
                },
                Ok(None) => {},
                Err(MetadataRefineError1(MetadataRefineError::Syntax, loc)) => {
                    self.add_syntax_error(&loc, DiagnosticKind::UnrecognizedMetadataSyntax, diagarg![]);
                },
            }
        }
        if self.peek(Token::BlockOpen) {
            if let Some((q, constant_name)) = exp.to_configuration_identifier_no_metadata() {
                self.push_location(&exp.location());
                let block = self.parse_block(context);
                return Some((Rc::new(Directive::ConfigurationDirective(ConfigurationDirective {
                    location: self.pop_location(),
                    namespace: q,
                    constant_name,
                    directive: Rc::new(Directive::Block(block)),
                })), true));
            }
        }
        None
    }

    fn parse_block(&mut self, context: ParserDirectiveContext) -> Block {
        self.mark_location();
        self.non_greedy_expect(Token::BlockOpen);
        let mut directives = vec![];
        if !self.expecting_token_error {
            let mut semicolon = false;
            while !self.peek(Token::BlockClose) && !self.peek(Token::Eof) {
                if !directives.is_empty() && !semicolon {
                    self.non_greedy_expect_virtual_semicolon();
                }
                let (directive, semicolon_1) = self.parse_directive(context.clone());
                directives.push(directive);
                semicolon = semicolon_1;
            }
            self.non_greedy_expect(Token::BlockClose);
        }
        Block { 
            location: self.pop_location(),
            directives,
        }
    }

    fn parse_if_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        let context = context.override_control_context(true, ParserControlFlowContext {
            breakable: true,
            iteration: false,
        });
        self.mark_location();
        self.next();
        let mut test = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        let mut consequent: Rc<Directive> = self.create_invalidated_directive(&self.tokenizer.cursor_location());
        let mut alternative: Option<Rc<Directive>> = None;
        let semicolon;
        self.non_greedy_expect(Token::ParenOpen);
        if self.expecting_token_error {
            semicolon = self.parse_semicolon();
        } else {
            test = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
            consequent = self.create_invalidated_directive(&self.tokenizer.cursor_location());
            self.non_greedy_expect(Token::ParenClose);
            if self.expecting_token_error {
                semicolon = self.parse_semicolon();
            } else {
                let (consequent_1, semicolon_1) = self.parse_substatement(context.clone());
                consequent = consequent_1;
                if self.peek(Token::Else) {
                    if !semicolon_1 {
                        self.non_greedy_expect_virtual_semicolon();
                    }
                    self.next();
                    let (alternative_2, semicolon_2) = self.parse_substatement(context.clone());
                    alternative = Some(alternative_2);
                    semicolon = semicolon_2;
                } else {
                    semicolon = semicolon_1;
                }
            }
        }
        (Rc::new(Directive::IfStatement(IfStatement {
            location: self.pop_location(),
            test, consequent, alternative,
        })), semicolon)
    }

    fn parse_switch_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        self.mark_location();
        self.next();
        if self.peek_context_keyword("type") {
            self.forbid_line_break_before_token();
            self.next();
            return self.parse_switch_type_statement(context);
        }
        let context = context.override_control_context(false, ParserControlFlowContext {
            breakable: true,
            iteration: false,
        });
        let mut discriminant = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        let mut cases: Vec<Case> = vec![];
        self.non_greedy_expect(Token::ParenOpen);
        if !self.expecting_token_error {
            discriminant = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
            self.non_greedy_expect(Token::ParenClose);
            if !self.expecting_token_error {
                self.non_greedy_expect(Token::BlockOpen);
                if !self.expecting_token_error {
                    cases = self.parse_case_elements(context);
                    self.non_greedy_expect(Token::BlockClose);
                }
            }
        }
        (Rc::new(Directive::SwitchStatement(SwitchStatement {
            location: self.pop_location(),
            discriminant, cases,
        })), true)
    }

    fn parse_case_elements(&mut self, context: ParserDirectiveContext) -> Vec<Case> {
        let mut cases = vec![];
        let mut semicolon = false;
        while !self.peek(Token::BlockClose) {
            if !cases.is_empty() && !semicolon {
                self.non_greedy_expect_virtual_semicolon();
            }
            if !(self.peek(Token::Case) || self.peek(Token::Default)) {
                break;
            }
            self.mark_location();
            let mut labels = vec![];
            loop {
                if self.peek(Token::Case) {
                    self.mark_location();
                    self.next();
                    let exp = self.parse_expression(ParserExpressionContext {
                        allow_in: true,
                        min_precedence: OperatorPrecedence::List,
                        ..default()
                    });
                    self.non_greedy_expect(Token::Colon);
                    labels.push(CaseLabel::Case((exp, self.pop_location())));
                } else if self.peek(Token::Default) {
                    self.mark_location();
                    self.next();
                    self.non_greedy_expect(Token::Colon);
                    labels.push(CaseLabel::Default(self.pop_location()));
                } else {
                    break;
                }
            }
            let mut directives = vec![];
            semicolon = false;
            while !(self.peek(Token::BlockClose) || self.peek(Token::Case) || self.peek(Token::Default)) {
                if !directives.is_empty() && !semicolon {
                    self.non_greedy_expect_virtual_semicolon();
                }
                let (directive, semicolon_1) = self.parse_directive(context.clone());
                directives.push(directive);
                semicolon = semicolon_1;
            }
            cases.push(Case {
                location: self.pop_location(),
                labels,
                directives,
            });
        }
        cases
    }

    fn parse_switch_type_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        let context = context.override_control_context(true, ParserControlFlowContext {
            breakable: true,
            iteration: false,
        });
        let mut discriminant = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        let mut cases: Vec<TypeCase> = vec![];
        self.non_greedy_expect(Token::ParenOpen);
        if !self.expecting_token_error {
            discriminant = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
            self.non_greedy_expect(Token::ParenClose);
            if !self.expecting_token_error {
                self.non_greedy_expect(Token::BlockOpen);
                if !self.expecting_token_error {
                    cases = self.parse_type_case_elements(context);
                    self.non_greedy_expect(Token::BlockClose);
                }
            }
        }
        (Rc::new(Directive::SwitchTypeStatement(SwitchTypeStatement {
            location: self.pop_location(),
            discriminant, cases,
        })), true)
    }

    fn parse_type_case_elements(&mut self, context: ParserDirectiveContext) -> Vec<TypeCase> {
        let mut cases = vec![];
        while !self.peek(Token::BlockClose) && !self.peek(Token::Eof) {
            if self.peek(Token::Default) {
                self.mark_location();
                self.next();
                let block = Rc::new(self.parse_block(context.clone()));
                cases.push(TypeCase {
                    location: self.pop_location(),
                    parameter: None,
                    block,
                });
            } else {
                self.mark_location();
                self.non_greedy_expect(Token::Case);
                if !self.expecting_token_error {
                    self.non_greedy_expect(Token::ParenOpen);
                    if !self.expecting_token_error {
                        let parameter = Some(self.parse_typed_destructuring());
                        self.non_greedy_expect(Token::ParenClose);
                        if !self.expecting_token_error {
                            let block = Rc::new(self.parse_block(context.clone()));
                            cases.push(TypeCase {
                                location: self.pop_location(),
                                parameter,
                                block,
                            });
                        }
                    }
                }
            }
        }
        cases
    }

    fn parse_do_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        let context = context.override_control_context(false, ParserControlFlowContext {
            breakable: true,
            iteration: true,
        });
        self.mark_location();
        self.next();

        // Body
        let (body, semicolon_1) = self.parse_substatement(context);
        if !semicolon_1 {
            self.non_greedy_expect_virtual_semicolon();
        }

        let mut test = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        self.non_greedy_expect(Token::While);
        if !self.expecting_token_error {
            test = self.create_invalidated_expression(&self.tokenizer.cursor_location());
            self.non_greedy_expect(Token::ParenOpen);
            if !self.expecting_token_error {
                test = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
                self.non_greedy_expect(Token::ParenClose);
            }
        }

        let semicolon = self.parse_semicolon();
        (Rc::new(Directive::DoStatement(DoStatement {
            location: self.pop_location(),
            body, test,
        })), semicolon)
    }

    fn parse_while_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        let context = context.override_control_context(false, ParserControlFlowContext {
            breakable: true,
            iteration: true,
        });
        self.mark_location();
        self.next();

        // Test
        let mut test = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        let mut body = self.create_invalidated_directive(&self.tokenizer.cursor_location());
        let semicolon: bool;
        self.non_greedy_expect(Token::ParenOpen);
        if !self.expecting_token_error {
            test = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
            body = self.create_invalidated_directive(&self.tokenizer.cursor_location());
            self.non_greedy_expect(Token::ParenClose);
            if !self.expecting_token_error {
                let (body_1, semicolon_1) = self.parse_substatement(context);
                body = body_1;
                semicolon = semicolon_1;
            } else {
                semicolon = self.parse_semicolon();
            }
        } else {
            semicolon = self.parse_semicolon();
        }

        (Rc::new(Directive::WhileStatement(WhileStatement {
            location: self.pop_location(),
            test, body,
        })), semicolon)
    }

    /// Parses `for`, `for..in` or `for each`.
    fn parse_for_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        let context = context.override_control_context(false, ParserControlFlowContext {
            breakable: true,
            iteration: true,
        });
        self.mark_location();
        self.next();

        // `for each`
        if self.peek_context_keyword("each") {
            self.forbid_line_break_before_token();
            self.next();
            return self.parse_for_each_statement(context);
        }

        self.non_greedy_expect(Token::ParenOpen);
        if self.expecting_token_error {
            let body = self.create_invalidated_directive(&self.tokenizer.cursor_location());
            let semicolon = self.parse_semicolon();
            return (Rc::new(Directive::ForStatement(ForStatement {
                location: self.pop_location(),
                init: None, test: None, update: None, body,
            })), semicolon);
        }

        let init_variable = if self.peek(Token::Var) || self.peek(Token::Const) {
            Some(self.parse_simple_variable_definition(false))
        } else {
            None
        };

        if init_variable.is_some() && self.consume(Token::In) {
            return self.parse_for_in_statement_with_left_variable(context, init_variable.unwrap());
        }

        let mut init_exp = if init_variable.is_none() && !self.peek(Token::Semicolon) {
            self.parse_opt_expression(ParserExpressionContext {
                allow_in: false,
                min_precedence: OperatorPrecedence::Postfix,
                ..default()
            })
        } else {
            None
        };

        if init_exp.is_some() && self.consume(Token::In) {
            return self.parse_for_in_statement_with_left_exp(context, init_exp.unwrap());
        }

        if init_exp.is_none() && init_variable.is_none() && !self.peek(Token::Semicolon) {
            init_exp = Some(self.parse_expression(ParserExpressionContext {
                allow_in: false, min_precedence: OperatorPrecedence::List, ..default()
            }));
        } else if let Some(exp) = init_exp.as_ref() {
            init_exp = Some(self.parse_subexpressions(exp.clone(), ParserExpressionContext {
                allow_in: false, min_precedence: OperatorPrecedence::List, ..default()
            }));
        }

        let init = if let Some(exp) = init_exp.as_ref() {
            Some(ForInitializer::Expression(exp.clone()))
        } else if let Some(variable) = init_variable.as_ref() {
            Some(ForInitializer::VariableDefinition(Rc::new(variable.clone())))
        } else {
            None
        };

        self.non_greedy_expect(Token::Semicolon);
        let test = if self.peek(Token::Semicolon) {
            None
        } else {
            Some(self.parse_expression(ParserExpressionContext {
                allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
            }))
        };
        self.non_greedy_expect(Token::Semicolon);
        let update = if self.peek(Token::ParenClose) {
            None
        } else {
            Some(self.parse_expression(ParserExpressionContext {
                allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
            }))
        };
        self.non_greedy_expect(Token::ParenClose);

        // Body
        let (body, semicolon) = self.parse_substatement(context);

        (Rc::new(Directive::ForStatement(ForStatement {
            location: self.pop_location(),
            init, test, update, body,
        })), semicolon)
    }

    fn parse_for_each_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        self.non_greedy_expect(Token::ParenOpen);
        if self.expecting_token_error {
            let left = ForInBinding::Expression(self.create_invalidated_expression(&self.tokenizer.cursor_location()));
            let right = self.create_invalidated_expression(&self.tokenizer.cursor_location());
            let body = self.create_invalidated_directive(&self.tokenizer.cursor_location());
            let semicolon = self.parse_semicolon();
            return (Rc::new(Directive::ForInStatement(ForInStatement {
                location: self.pop_location(),
                each: true, left, right, body,
            })), semicolon);
        }

        let left = if self.peek(Token::Var) || self.peek(Token::Const) {
            self.mark_location();
            let kind = (if self.peek(Token::Var) { VariableDefinitionKind::Var } else { VariableDefinitionKind::Const }, self.token_location());
            self.next();
            let binding = self.parse_variable_binding(false);
            if let Some(init) = &binding.initializer {
                self.add_syntax_error(&init.location(), DiagnosticKind::IllegalForInInitializer, vec![]);
            }
            ForInBinding::VariableDefinition(Rc::new(SimpleVariableDefinition {
                location: self.pop_location(),
                kind,
                bindings: vec![Rc::new(binding)],
            }))
        } else {
            ForInBinding::Expression(self.parse_expression(ParserExpressionContext {
                allow_in: false, min_precedence: OperatorPrecedence::Postfix, ..default()
            }))
        };
        let mut right = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        self.non_greedy_expect(Token::In);
        if !self.expecting_token_error {
            right = self.parse_expression(ParserExpressionContext {
                allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
            });
        }
        self.non_greedy_expect(Token::ParenClose);

        // Body
        let (body, semicolon) = self.parse_substatement(context);

        (Rc::new(Directive::ForInStatement(ForInStatement {
            location: self.pop_location(),
            each: true, left, right, body,
        })), semicolon)
    }

    fn parse_for_in_statement_with_left_variable(&mut self, context: ParserDirectiveContext, left: SimpleVariableDefinition) -> (Rc<Directive>, bool) {
        let variable_binding = left.bindings[0].clone();

        if let Some(init) = &variable_binding.initializer {
            self.add_syntax_error(&init.location(), DiagnosticKind::IllegalForInInitializer, vec![]);
        }

        if left.bindings.len() > 1 {
            self.add_syntax_error(&left.kind.1.clone(), DiagnosticKind::MultipleForInBindings, vec![]);
        }

        let right = self.parse_expression(ParserExpressionContext {
            allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
        });
        self.non_greedy_expect(Token::ParenClose);

        // Body
        let (body, semicolon) = self.parse_substatement(context);

        (Rc::new(Directive::ForInStatement(ForInStatement {
            location: self.pop_location(),
            each: false, left: ForInBinding::VariableDefinition(Rc::new(left)), right, body,
        })), semicolon)
    }

    fn parse_for_in_statement_with_left_exp(&mut self, context: ParserDirectiveContext, left: Rc<Expression>) -> (Rc<Directive>, bool) {
        let right = self.parse_expression(ParserExpressionContext {
            allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
        });
        self.non_greedy_expect(Token::ParenClose);

        // Body
        let (body, semicolon) = self.parse_substatement(context);

        (Rc::new(Directive::ForInStatement(ForInStatement {
            location: self.pop_location(),
            each: false, left: ForInBinding::Expression(left), right, body,
        })), semicolon)
    }

    fn parse_simple_variable_definition(&mut self, allow_in: bool) -> SimpleVariableDefinition {
        self.mark_location();
        let kind: VariableDefinitionKind;
        let kind_location = self.token_location();
        if self.consume(Token::Const) {
            kind = VariableDefinitionKind::Const;
        } else {
            self.non_greedy_expect(Token::Var);
            kind = VariableDefinitionKind::Var;
        }
        let mut bindings = vec![Rc::new(self.parse_variable_binding(allow_in))];
        while self.consume(Token::Comma) {
            bindings.push(Rc::new(self.parse_variable_binding(allow_in)));
        }
        SimpleVariableDefinition {
            location: self.pop_location(),
            kind: (kind, kind_location),
            bindings,
        }
    }

    fn parse_with_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        let context = context.override_control_context(true, ParserControlFlowContext {
            breakable: true,
            iteration: false,
        });
        self.mark_location();
        self.next();

        let mut object = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        self.non_greedy_expect(Token::ParenOpen);
        if !self.expecting_token_error {
            object = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
        }
        self.non_greedy_expect(Token::ParenClose);

        // Body
        let (body, semicolon) = self.parse_substatement(context);

        (Rc::new(Directive::WithStatement(WithStatement {
            location: self.pop_location(),
            object, body,
        })), semicolon)
    }

    fn parse_break_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        self.mark_location();
        self.next();

        let label = if self.previous_token.1.line_break(&self.token.1) { None } else { self.consume_identifier(false) };
        let label_location = label.clone().map(|label| label.1.clone());
        let label = label.map(|label| label.0.clone());

        let semicolon = self.parse_semicolon();

        let node = Rc::new(Directive::BreakStatement(BreakStatement {
            location: self.pop_location(),
            label: label.clone().map(|l| (l.clone(), label_location.clone().unwrap())),
        }));

        if label.is_some() && !context.is_label_defined(label.clone().unwrap()) {
            self.add_syntax_error(&label_location.unwrap(), DiagnosticKind::UndefinedLabel, diagarg![label.clone().unwrap()]);
        } else if !context.is_break_allowed(label) {
            self.add_syntax_error(&node.location(), DiagnosticKind::IllegalBreak, vec![]);
        }

        (node, semicolon)
    }

    fn parse_continue_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        self.mark_location();
        self.next();

        let label = if self.previous_token.1.line_break(&self.token.1) { None } else { self.consume_identifier(false) };
        let label_location = label.clone().map(|label| label.1.clone());
        let label = label.map(|label| label.0.clone());

        let semicolon = self.parse_semicolon();

        let node = Rc::new(Directive::ContinueStatement(ContinueStatement {
            location: self.pop_location(),
            label: label.clone().map(|l| (l.clone(), label_location.clone().unwrap())),
        }));

        if label.is_some() && !context.is_label_defined(label.clone().unwrap()) {
            self.add_syntax_error(&label_location.unwrap(), DiagnosticKind::UndefinedLabel, diagarg![label.clone().unwrap()]);
        } else if !context.is_continue_allowed(label) {
            self.add_syntax_error(&node.location(), DiagnosticKind::IllegalContinue, vec![]);
        }

        (node, semicolon)
    }

    fn parse_return_statement(&mut self, _context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        self.mark_location();
        self.next();

        let expression = if self.previous_token.1.line_break(&self.token.1) { None } else {
            self.parse_opt_expression(ParserExpressionContext {
                allow_in: true,
                min_precedence: OperatorPrecedence::List,
                ..default()
            })
        };

        let semicolon = self.parse_semicolon();

        let node = Rc::new(Directive::ReturnStatement(ReturnStatement {
            location: self.pop_location(),
            expression,
        }));

        (node, semicolon)
    }

    fn parse_throw_statement(&mut self, _context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        self.mark_location();
        self.next();

        let line_break = self.previous_token.1.line_break(&self.token.1);

        let expression = self.parse_expression(ParserExpressionContext {
            allow_in: true,
            min_precedence: OperatorPrecedence::List,
            ..default()
        });

        if line_break {
            self.add_syntax_error(&expression.location(), DiagnosticKind::ExpressionMustNotFollowLineBreak, vec![]);
        }

        let semicolon = self.parse_semicolon();

        let node = Rc::new(Directive::ThrowStatement(ThrowStatement {
            location: self.pop_location(),
            expression,
        }));

        (node, semicolon)
    }

    fn parse_try_statement(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        self.mark_location();
        self.next();
        let context = context.clone_control();
        let block = Rc::new(self.parse_block(context.clone()));
        let mut catch_clauses: Vec<CatchClause> = vec![];
        let mut finally_clause: Option<FinallyClause> = None;
        let mut found_catch = false;
        loop {
            if self.peek(Token::Catch) {
                found_catch = true;
                self.mark_location();
                self.next();
                self.non_greedy_expect(Token::ParenOpen);
                if !self.expecting_token_error {
                    let parameter = self.parse_typed_destructuring();
                    self.non_greedy_expect(Token::ParenClose);
                    if !self.expecting_token_error {
                        let block = Rc::new(self.parse_block(context.clone()));
                        catch_clauses.push(CatchClause {
                            location: self.pop_location(),
                            parameter,
                            block,
                        });
                    }
                }
            } else if self.peek(Token::Finally) {
                self.mark_location();
                self.next();
                let block = Rc::new(self.parse_block(context.clone()));
                finally_clause = Some(FinallyClause {
                    location: self.pop_location(),
                    block,
                });
                break;
            } else {
                break;
            }
        }
        if !found_catch && finally_clause.is_none() {
            self.non_greedy_expect(Token::Catch);
        }

        let node = Rc::new(Directive::TryStatement(TryStatement {
            location: self.pop_location(),
            block, catch_clauses, finally_clause,
        }));

        (node, true)
    }

    fn parse_default_xml_namespace_statement(&mut self) -> (Rc<Directive>, bool) {
        self.mark_location();
        self.next();

        let mut expression = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        self.forbid_line_break_before_token();
        self.non_greedy_expect_context_keyword("xml");
        if !self.expecting_token_error {
            expression = self.create_invalidated_expression(&self.tokenizer.cursor_location());
            self.forbid_line_break_before_token();
            self.non_greedy_expect_context_keyword("namespace");
            if !self.expecting_token_error {
                expression = self.create_invalidated_expression(&self.tokenizer.cursor_location());
                self.non_greedy_expect(Token::Assign);

                if !self.expecting_token_error {
                    expression = self.parse_expression(ParserExpressionContext {
                        allow_in: true,
                        allow_assignment: false,
                        min_precedence: OperatorPrecedence::AssignmentAndOther,
                        ..default()
                    });
                }
            }
        }

        let semicolon = self.parse_semicolon();

        let node = Rc::new(Directive::DefaultXmlNamespaceStatement(DefaultXmlNamespaceStatement {
            location: self.pop_location(),
            right: expression,
        }));

        (node, semicolon)
    }

    fn forbid_line_break_before_token(&mut self) {
        if self.previous_token.1.line_break(&self.token.1) {
            self.add_syntax_error(&self.token.1.clone(), DiagnosticKind::TokenMustNotFollowLineBreak, vec![]);
        }
    }

    fn parse_directive(&mut self, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        let asdoc: Option<Rc<Asdoc>> = if self.peek(Token::SquareOpen) { None } else { self.parse_asdoc() };
        // ConfigurationDirective or Statement
        if let Token::Identifier(id) = &self.token.0 {
            let id = (id.clone(), self.token_location());
            self.next();

            if id.0 == "include" && id.1.character_count() == "include".len() && matches!(self.token.0, Token::String(_)) && !self.previous_token.1.line_break(&self.token.1) {
                return self.parse_include_directive(context, id.1);
            }

            // Labeled statement
            if self.consume(Token::Colon) {
                self.push_location(&id.1);
                let (substatement, semicolon) = self.parse_substatement(context.put_label(id.0.clone()));
                let labeled = Rc::new(Directive::LabeledStatement(LabeledStatement {
                    location: self.pop_location(),
                    label: id.clone(),
                    substatement,
                }));
                return (labeled, semicolon);
            }

            // If there is a line break or offending token is "::",
            // do not proceed into parsing an expression attribute or annotatble directive.
            let eligible_attribute_or_directive
                =  !self.previous_token.1.line_break(&self.token.1)
                && !(matches!(self.token.0, Token::ColonColon));

            if eligible_attribute_or_directive && (self.peek_annotatable_directive_identifier_name() || self.lookbehind_is_annotatable_directive_identifier_name()) {
                let mut context1: AnnotatableContext;

                if ["enum", "type", "namespace"].contains(&id.0.as_ref())
                && id.1.character_count() == id.0.len()
                && self.token.0.is_identifier_name() {
                    context1 = AnnotatableContext {
                        start_location: id.1.clone(),
                        asdoc,
                        attributes: vec![],
                        context: context.clone(),
                        directive_context_keyword: Some(id.clone()),
                    };
                    // self.parse_attribute_keywords_or_expressions(&mut context);
                } else {
                    let mut first_attr_expr = self.parse_expression_starting_with_identifier(id);
                    first_attr_expr = self.parse_subexpressions(first_attr_expr, ParserExpressionContext {
                        allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
                    });

                    // Do not proceed into parsing an annotatable directive
                    // if there is a line break after an expression attribute,
                    // or if the offending token is not an identifier name,
                    // or if the expression attribute is not a valid access modifier.
                    if !first_attr_expr.valid_access_modifier() || self.previous_token.1.line_break(&self.token.1) || !(matches!(self.token.0, Token::Identifier(_)) || self.token.0.is_reserved_word()) {
                        self.push_location(&first_attr_expr.location());

                        // Parse CONFIG::VAR_NAME
                        if let Some(result) = self.parse_opt_config(&first_attr_expr, asdoc.clone(), context.clone()) {
                            return result;
                        }

                        let semicolon = self.parse_semicolon();
                        return (Rc::new(Directive::ExpressionStatement(ExpressionStatement {
                            location: self.pop_location(),
                            expression: first_attr_expr,
                        })), semicolon);
                    }

                    let first_attr = self.keyword_or_expression_attribute_from_expression(&first_attr_expr);

                    context1 = AnnotatableContext {
                        start_location: first_attr.location(),
                        asdoc,
                        attributes: vec![first_attr],
                        context: context.clone(),
                        directive_context_keyword: None,
                    };
                    self.parse_attribute_keywords_or_expressions(&mut context1);
                }
                return self.parse_annotatable_directive(context1);
            } else if self.peek(Token::ColonColon) {
                self.parse_qualified_identifier_statement_or_config(context, id, asdoc)
            } else {
                self.parse_statement_starting_with_identifier(context, id)
            }
        } else if self.peek(Token::Import) {
            self.parse_import_directive_or_expression_statement(context)
        } else if self.peek(Token::SquareOpen) {
            self.mark_location();
            let exp = self.parse_expression(ParserExpressionContext {
                allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
            });
            if self.peek_annotatable_directive_identifier_name() {
                match exp.to_metadata(self) {
                    Ok(Some(metadata)) => {
                        let mut context = AnnotatableContext {
                            start_location: self.pop_location(),
                            asdoc: self.parse_asdoc(),
                            attributes: metadata,
                            context: context.clone(),
                            directive_context_keyword: None,
                        };
                        self.parse_attribute_keywords_or_expressions(&mut context);
                        return self.parse_annotatable_directive(context);
                    },
                    Ok(None) => {},
                    Err(MetadataRefineError1(MetadataRefineError::Syntax, loc)) => {
                        self.add_syntax_error(&loc, DiagnosticKind::UnrecognizedMetadataSyntax, diagarg![]);
                    },
                }
            }
            let semicolon = self.parse_semicolon();
            (Rc::new(Directive::ExpressionStatement(ExpressionStatement {
                location: self.pop_location(),
                expression: exp,
            })), semicolon)
        } else if self.peek(Token::Public) || self.peek(Token::Private) || self.peek(Token::Protected)
        || self.peek(Token::Internal) || self.peek(Token::Var) || self.peek(Token::Const)
        || self.peek(Token::Function) || self.peek(Token::Class) || self.peek(Token::Interface) {
            let is_public = self.peek(Token::Public);
            let rns = self.parse_opt_reserved_namespace();
            let mut attributes: Vec<Attribute> = vec![];
            if let Some(rns) = rns {
                // The public += ns.*; directive
                if self.peek(Token::AddAssign) && is_public {
                    return self.parse_package_concat_directive(&rns.location(), context);
                }

                // Do not proceed into parsing an annotatable directive
                // if there is a "::" token.
                if matches!(self.token.0, Token::ColonColon) {
                    self.push_location(&rns.location());
                    let rns = Rc::new(Expression::QualifiedIdentifier(self.finish_qualified_identifier(false, rns.location(), rns)));
                    let rns = self.parse_subexpressions(rns, ParserExpressionContext {
                        allow_in: true, min_precedence: OperatorPrecedence::List, ..default()
                    });
                    let semicolon = self.parse_semicolon();
                    return (Rc::new(Directive::ExpressionStatement(ExpressionStatement {
                        location: self.pop_location(),
                        expression: rns,
                    })), semicolon);
                }
                attributes.push(rns.to_reserved_namespace_attribute().unwrap());
            }
            let mut context = AnnotatableContext {
                start_location: self.token_location(),
                asdoc,
                attributes,
                context: context.clone(),
                directive_context_keyword: None,
            };
            self.parse_attribute_keywords_or_expressions(&mut context);
            return self.parse_annotatable_directive(context);
        } else if self.peek(Token::Use) {
            self.parse_use_namespace_directive()
        } else {
            let i = self.tokenizer.characters().index();
            let r = self.parse_statement(context);
            if i == self.tokenizer.characters().index() {
                self.patch_syntax_error(DiagnosticKind::ExpectingStatement, DiagnosticKind::ExpectingDirective, diagarg![self.token.0.clone()]);
            }
            r
        }
    }

    fn parse_directives(&mut self, context: ParserDirectiveContext) -> Vec<Rc<Directive>> {
        let mut directives = vec![];
        let mut semicolon = false;
        while !self.peek(Token::Eof) {
            if !directives.is_empty() && !semicolon {
                self.non_greedy_expect_virtual_semicolon();
            }
            let (directive, semicolon_1) = self.parse_directive(context.clone());
            directives.push(directive);
            semicolon = semicolon_1;
        }
        directives
    }

    fn parse_expression_attribute(&mut self, id: &(String, Location)) -> Rc<Expression> {
        let mut result = Rc::new(Expression::QualifiedIdentifier(QualifiedIdentifier {
            location: id.1.clone(),
            attribute: false,
            qualifier: None,
            id: QualifiedIdentifierIdentifier::Id(id.clone()),
        }));
        loop {
            if self.peek(Token::Dot) {
                self.push_location(&result.location());
                self.next();
                let id = self.parse_qualified_identifier();
                result = Rc::new(Expression::Member(MemberExpression {
                    location: self.pop_location(),
                    base: result, identifier: id
                }));
            } else if self.consume(Token::SquareOpen) {
                self.push_location(&result.location());
                let key = self.parse_expression(ParserExpressionContext { allow_in: true, min_precedence: OperatorPrecedence::List, ..default() });
                self.non_greedy_expect(Token::SquareClose);
                result = Rc::new(Expression::ComputedMember(ComputedMemberExpression {
                    base: result, asdoc: None, key, location: self.pop_location()
                }));
            } else {
                break;
            }
        }
        result
    }

    fn report_modifier_errors(&self, context: &AnnotatableContext) {
        let mut i = 0usize;
        while i < context.attributes.len() {
            let a = &context.attributes[i];
            if Attribute::has(&context.attributes[..i], &a) {
                self.add_syntax_error(&a.location(), DiagnosticKind::DuplicateAttribute, diagarg![]);
            }
            if Attribute::is_duplicate_access_modifier(&context.attributes[..i], &a) {
                self.add_syntax_error(&a.location(), DiagnosticKind::DuplicateAccessModifier, diagarg![]);
            }
            i += 1;
        }
    }

    fn parse_annotatable_directive(&mut self, context: AnnotatableContext) -> (Rc<Directive>, bool) {
        if self.peek(Token::Var) || self.peek(Token::Const) {
            self.report_modifier_errors(&context);
            self.parse_variable_definition(context)
        } else if self.consume(Token::Function) {
            self.report_modifier_errors(&context);
            self.parse_function_definition(context)
        } else if self.consume(Token::Class) {
            self.report_modifier_errors(&context);
            self.parse_class_definition(context)
        } else if context.has_directive_context_keyword("enum") {
            self.report_modifier_errors(&context);
            self.parse_enum_definition(context)
        } else if context.has_directive_context_keyword("namespace") {
            self.report_modifier_errors(&context);
            self.parse_namespace_definition(context)
        } else if self.consume(Token::Interface) {
            self.report_modifier_errors(&context);
            self.parse_interface_definition(context)
        } else if context.has_directive_context_keyword("type") {
            self.report_modifier_errors(&context);
            self.parse_type_definition(context)
        } else {
            // In case there is a series of inline modifiers,
            // report semicolon error between each.
            let mut i = 0usize;
            let mut error = false;
            while i < context.attributes.len() {
                if !context.attributes[i].is_metadata() {
                    let loc1 = context.attributes[i].location();
                    if i + 1 < context.attributes.len() {
                        let loc2 = context.attributes[i + 1].location();
                        if !loc1.line_break(&loc2) {
                            self.add_syntax_error(&loc2, DiagnosticKind::ExpectingEitherSemicolonOrNewLineHere, vec![]);
                            error = true;
                        }
                    }
                }
                i += 1;
            }

            if !error {
                self.add_syntax_error(&self.token_location(), DiagnosticKind::ExpectingDirectiveKeyword, diagarg![self.token.0.clone()]);
            }
            self.push_location(&context.start_location);
            let loc = self.pop_location();
            (self.create_invalidated_directive(&loc), true)
        }
    }

    pub(crate) fn refine_metadata(&self, exp: &Rc<Expression>, asdoc: Option<Rc<Asdoc>>) -> Result<Rc<Metadata>, MetadataRefineError> {
        if let Expression::Call(CallExpression { base, arguments, .. }) = exp.as_ref() {
            let Ok(name) = self.refine_metadata_name(base) else {
                return Err(MetadataRefineError::Syntax);
            };
            Ok(Rc::new(Metadata {
                location: exp.location(),
                asdoc,
                name,
                entries: Some(self.refine_metadata_entries(arguments)?),
            }))
        } else {
            if let Ok(name) = self.refine_metadata_name(exp) {
                Ok(Rc::new(Metadata {
                    location: exp.location(),
                    asdoc,
                    name,
                    entries: None,
                }))
            } else {
                Err(MetadataRefineError::Syntax)
            }
        }
    }

    fn refine_metadata_name(&self, exp: &Rc<Expression>) -> Result<(String, Location), MetadataRefineError> {
        if let Expression::QualifiedIdentifier(id) = exp.as_ref() {
            if id.attribute {
                return Err(MetadataRefineError::Syntax);
            }
            let qual = id.qualifier.as_ref().and_then(|q| q.to_identifier_name().map(|n| n.0));
            if id.qualifier.is_some() && qual.is_none() {
                return Err(MetadataRefineError::Syntax);
            }
            if let QualifiedIdentifierIdentifier::Id((s, _)) = &id.id {
                if s == "*" { Err(MetadataRefineError::Syntax) } else { Ok((if let Some(q) = qual { format!("{q}::{s}") } else { s.to_string() }, exp.location())) }
            } else {
                Err(MetadataRefineError::Syntax)
            }
        } else {
            Err(MetadataRefineError::Syntax)
        }
    }

    fn refine_metadata_entries(&self, list: &Vec<Rc<Expression>>) -> Result<Vec<Rc<MetadataEntry>>, MetadataRefineError> {
        let mut r = Vec::<Rc<MetadataEntry>>::new();
        for entry in list {
            r.push(self.refine_metadata_entry(&entry)?);
        }
        Ok(r)
    }

    fn refine_metadata_entry(&self, exp: &Rc<Expression>) -> Result<Rc<MetadataEntry>, MetadataRefineError> {
        match exp.as_ref() {
            Expression::Assignment(AssignmentExpression { compound, left, right, location }) => {
                if compound.is_some() {
                    return Err(MetadataRefineError::Syntax);
                }
                let key = self.refine_metadata_name(left)?;
                if matches!(right.as_ref(), Expression::QualifiedIdentifier(_)) {
                    return Err(MetadataRefineError::Syntax);
                }
                let value = self.refine_metadata_value(right)?;
                Ok(Rc::new(MetadataEntry {
                    location: location.clone(),
                    key: Some(key),
                    value: Rc::new(value),
                }))
            },
            _ => {
                let value = self.refine_metadata_value(exp)?;
                Ok(Rc::new(MetadataEntry {
                    location: exp.location(),
                    key: None,
                    value: Rc::new(value),
                }))
            },
        }
    }

    fn refine_metadata_value(&self, exp: &Rc<Expression>) -> Result<MetadataValue, MetadataRefineError> {
        match exp.as_ref() {
            Expression::QualifiedIdentifier(_) => {
                let name = self.refine_metadata_name(&exp)?;
                Ok(MetadataValue::IdentifierString(name))
            },
            Expression::StringLiteral(StringLiteral { value, .. }) => Ok(MetadataValue::String((value.clone(), exp.location()))),
            _ => Err(MetadataRefineError::Syntax),
        }
    }

    fn parse_package_concat_directive(&mut self, start: &Location, context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        self.push_location(start);
        self.next();
        let mut package_name: Vec<(String, Location)> = vec![self.expect_identifier(false)];
        let mut import_specifier = ImportSpecifier::Wildcard(self.token_location());

        if !self.peek(Token::Dot) {
            self.non_greedy_expect(Token::Dot);
        }

        while self.consume(Token::Dot) {
            if self.peek(Token::Times) {
                import_specifier = ImportSpecifier::Wildcard(self.token_location());
                self.next();
                break;
            } else if self.peek(Token::Power) {
                import_specifier = ImportSpecifier::Recursive(self.token_location());
                self.next();
                break;
            } else {
                let id1 = self.expect_identifier(true);
                if !self.peek(Token::Dot) {
                    import_specifier = ImportSpecifier::Identifier(id1.clone());
                    break;
                } else {
                    package_name.push(id1.clone());
                }
            }
        }

        let semicolon = self.parse_semicolon();

        let node = Rc::new(Directive::PackageConcatDirective(PackageConcatDirective {
            location: self.pop_location(),
            package_name,
            import_specifier,
        }));

        if !(matches!(context, ParserDirectiveContext::PackageBlock)) {
            self.add_syntax_error(&node.location(), DiagnosticKind::UnexpectedDirective, vec![]);
        }

        (node, semicolon)
    }

    fn parse_import_directive_or_expression_statement(&mut self, _context: ParserDirectiveContext) -> (Rc<Directive>, bool) {
        self.mark_location();
        self.next();
        if self.consume(Token::Dot) {
            self.duplicate_location();
            self.non_greedy_expect_context_keyword("meta");
            let mut expression = Rc::new(Expression::ImportMeta(ImportMeta {
                location: self.pop_location(),
            }));
            expression = self.parse_subexpressions(expression, ParserExpressionContext {
                allow_in: true,
                min_precedence: OperatorPrecedence::List,
                ..default()
            });
            let semicolon = self.parse_semicolon();
            (Rc::new(Directive::ExpressionStatement(ExpressionStatement {
                location: self.pop_location(),
                expression,
            })), semicolon)
        } else {
            let mut alias: Option<(String, Location)> = None;
            let mut package_name: Vec<(String, Location)> = vec![];
            let mut import_specifier = ImportSpecifier::Wildcard(self.token_location());
            let id1 = self.expect_identifier(false);
            if self.consume(Token::Assign) {
                alias = Some(id1.clone());
                package_name.push(self.expect_identifier(false));
            } else {
                package_name.push(id1);
            }
    
            if !self.peek(Token::Dot) {
                self.non_greedy_expect(Token::Dot);
            }
    
            while self.consume(Token::Dot) {
                if self.peek(Token::Times) {
                    import_specifier = ImportSpecifier::Wildcard(self.token_location());
                    self.next();
                    break;
                } else if self.peek(Token::Power) {
                    import_specifier = ImportSpecifier::Recursive(self.token_location());
                    self.next();
                    break;
                } else {
                    let id1 = self.expect_identifier(true);
                    if !self.peek(Token::Dot) {
                        import_specifier = ImportSpecifier::Identifier(id1.clone());
                        break;
                    } else {
                        package_name.push(id1.clone());
                    }
                }
            }
    
            let semicolon = self.parse_semicolon();
    
            let node = Rc::new(Directive::ImportDirective(ImportDirective {
                location: self.pop_location(),
                alias,
                package_name,
                import_specifier,
            }));
    
            (node, semicolon)
        }
    }

    fn parse_include_directive(&mut self, context: ParserDirectiveContext, start: Location) -> (Rc<Directive>, bool) {
        self.push_location(&start);
        let source_path_location = self.token_location();
        let Token::String(source) = &self.token.0.clone() else {
            panic!();
        };
        let source = source.clone();
        self.next();
        let semicolon = self.parse_semicolon();

        let nested_compilation_unit: Rc<CompilationUnit>;

        // Select origin file path
        let origin_file_path = if let Some(file_path) = self.tokenizer.compilation_unit().file_path.clone() {
            Some(file_path)
        } else {
            std::env::current_dir().ok().map(|d| d.to_string_lossy().into_owned())
        };

        // Resolve source
        if let Some(origin_file_path) = origin_file_path {
            let sub_flex_file_path = hydroperfox_filepaths::FlexPath::from_n_native([origin_file_path.as_ref(), "..", source.as_ref()]);
            let sub_file_path = sub_flex_file_path.to_string_with_flex_separator();

            if !sub_flex_file_path.has_extension(".include.as") {
                self.add_syntax_error(&source_path_location.clone(), DiagnosticKind::UnexpectedIncludeExtension, vec![]);

                // Use a placeholder compilation unit
                nested_compilation_unit = CompilationUnit::new(None, "".into());
            } else if self.tokenizer.compilation_unit().include_directive_is_circular(&sub_file_path) {
                self.add_syntax_error(&source_path_location.clone(), DiagnosticKind::CircularIncludeDirective, vec![]);

                // Use a placeholder compilation unit
                nested_compilation_unit = CompilationUnit::new(None, "".into());
            } else {
                if let Ok(content) = std::fs::read_to_string(&sub_file_path) {
                    nested_compilation_unit = CompilationUnit::new(Some(sub_file_path.clone()), content);
                } else {
                    self.add_syntax_error(&source_path_location.clone(), DiagnosticKind::FailedToIncludeFile, vec![]);

                    // Use a placeholder compilation unit
                    nested_compilation_unit = CompilationUnit::new(None, "".into());
                }
            }
        } else {
            self.add_syntax_error(&source_path_location.clone(), DiagnosticKind::ParentSourceIsNotAFile, vec![]);

            // Use a placeholder compilation unit
            nested_compilation_unit = CompilationUnit::new(None, "".into());
        }

        // Inherit compiler options
        nested_compilation_unit.set_compiler_options(self.tokenizer.compilation_unit().compiler_options());

        // Add sub compilation unit to super compilation unit
        self.tokenizer.compilation_unit().add_nested_compilation_unit(nested_compilation_unit.clone());

        // Parse directives from replacement source
        let (nested_packages, nested_directives) = parse_include_directive_source(nested_compilation_unit.clone(), context);

        // Delegate sub compilation unit errors to super compilation unit
        if nested_compilation_unit.invalidated() {
            self.tokenizer.compilation_unit().invalidated.set(true);
        }

        let node = Rc::new(Directive::IncludeDirective(IncludeDirective {
            location: self.pop_location(),
            source,
            nested_packages,
            nested_directives,
            nested_compilation_unit: nested_compilation_unit.clone(),
        }));

        (node, semicolon)
    }

    fn parse_use_namespace_directive(&mut self) -> (Rc<Directive>, bool) {
        self.mark_location();
        self.next();
        let mut expression = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        self.non_greedy_expect_context_keyword("namespace");
        if !self.expecting_token_error {
            expression = self.parse_expression(ParserExpressionContext {
                min_precedence: OperatorPrecedence::List,
                ..default()
            });
        }
        let semicolon = self.parse_semicolon();

        let node = Rc::new(Directive::UseNamespaceDirective(UseNamespaceDirective {
            location: self.pop_location(),
            expression,
        }));

        (node, semicolon)
    }

    fn parse_variable_definition(&mut self, context: AnnotatableContext) -> (Rc<Directive>, bool) {
        let AnnotatableContext { start_location, asdoc, attributes, context, .. } = context;
        let has_static = Attribute::find_static(&attributes).is_some();
        self.push_location(&start_location);
        let kind_location = self.token_location();
        let kind = if self.consume(Token::Const) {
            VariableDefinitionKind::Const
        } else {
            self.non_greedy_expect(Token::Var);
            VariableDefinitionKind::Var
        };
        let mut bindings = vec![Rc::new(self.parse_variable_binding(true))];
        while self.consume(Token::Comma) {
            bindings.push(Rc::new(self.parse_variable_binding(true)));
        }

        // Forbid destructuring bindings in enumerations.
        if !has_static && matches!(context, ParserDirectiveContext::EnumBlock) {
            if kind != VariableDefinitionKind::Const {
                self.add_syntax_error(&kind_location, DiagnosticKind::EnumMembersMustBeConst, diagarg![]);
            }
            for binding in &bindings {
                let malformed = !matches!(binding.destructuring.destructuring.as_ref(), Expression::QualifiedIdentifier(_))
                    || binding.destructuring.type_annotation.is_some();
                if malformed {
                    self.add_syntax_error(&binding.location(), DiagnosticKind::MalformedEnumMember, diagarg![]);
                }
            }
        }

        for a in &attributes {
            if a.is_metadata() {
                continue;
            }
            match a {
                Attribute::Static(_) => {
                    if !context.is_type_block() {
                        // Unallowed attribute
                        self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
                    }
                },
                Attribute::Expression(_) |
                Attribute::Public(_) |
                Attribute::Private(_) |
                Attribute::Protected(_) |
                Attribute::Internal(_) => {
                    self.verify_visibility(&a, &context);
                },
                _ => {
                    // Unallowed attribute
                    self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
                },
            }
        }

        let semicolon = self.parse_semicolon();
        let node = Rc::new(Directive::VariableDefinition(VariableDefinition {
            location: self.pop_location(),
            asdoc,
            attributes,
            kind: (kind, kind_location),
            bindings,
        }));

        (node, semicolon)
    }

    fn parse_function_definition(&mut self, context: AnnotatableContext) -> (Rc<Directive>, bool) {
        let AnnotatableContext { start_location, asdoc, attributes, context, .. } = context;
        let has_native = Attribute::find_native(&attributes).is_some();
        let has_abstract = Attribute::find_abstract(&attributes).is_some();
        self.push_location(&start_location);
        let mut name = self.expect_identifier(true);
        let mut getter = false;
        let mut setter = false;
        if self.peek_identifier(true).is_some() {
            getter = Token::is_context_keyword(&self.previous_token, "get");
            setter = Token::is_context_keyword(&self.previous_token, "set");
            if getter || setter {
                name = self.expect_identifier(true);
            }
        }
        let constructor = !getter && !setter && context.function_name_is_constructor(&name);
        let name = if getter {
            FunctionName::Getter(name)
        } else if setter {
            FunctionName::Setter(name)
        } else if constructor {
            FunctionName::Constructor(name)
        } else {
            FunctionName::Identifier(name)
        };
        let block_context = if constructor {
            ParserDirectiveContext::ConstructorBlock { super_statement_found: Rc::new(Cell::new(false)) }
        } else {
            ParserDirectiveContext::Default
        };
        let common = self.parse_function_common(false, block_context, true);
        let semicolon = if common.has_block_body() { true } else { self.parse_semicolon() };

        /*
        if constructor && common.signature.result_type.is_some() {
            self.add_syntax_error(&name.location(), DiagnosticKind::ConstructorMustNotSpecifyResultType, diagarg![]);
        }
        */

        // Not all kinds of functions may be generators.
        if common.contains_yield && (constructor || getter || setter) {
            self.add_syntax_error(&name.location(), DiagnosticKind::FunctionMayNotBeGenerator, diagarg![]);
        }

        // Not all kinds of functions may be asynchronous.
        if common.contains_await && (constructor || getter || setter) {
            self.add_syntax_error(&name.location(), DiagnosticKind::FunctionMayNotBeAsynchronous, diagarg![]);
        }

        let interface_method = matches!(context, ParserDirectiveContext::InterfaceBlock);

        // Body verification.
        //
        // Note that interface methods must never have a body unlike in Java.
        if (interface_method || has_native || has_abstract) && common.body.is_some() {
            self.add_syntax_error(&name.location(), DiagnosticKind::FunctionMustNotContainBody, diagarg![]);
        } else if !(interface_method || has_native || has_abstract) && common.body.is_none() {
            self.add_syntax_error(&name.location(), DiagnosticKind::FunctionMustContainBody, diagarg![]);
        }

        // Interface methods must not contain any annotations except for meta-data.
        if !attributes.is_empty() && interface_method {
            if !attributes.last().unwrap().is_metadata() {
                self.add_syntax_error(&name.location(), DiagnosticKind::FunctionMustNotContainAnnotations, diagarg![]);
            }
        }

        for a in &attributes {
            if a.is_metadata() {
                continue;
            }
            match a {
                Attribute::Static(_) => {
                    if !context.is_type_block() {
                        // Unallowed attribute
                        self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
                    }
                },
                Attribute::Final(_) |
                Attribute::Override(_) |
                Attribute::Abstract(_) => {
                    if !context.is_type_block() || constructor {
                        // Unallowed attribute
                        self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
                    }
                },

                Attribute::Native(_) => {},

                Attribute::Expression(_) |
                Attribute::Public(_) |
                Attribute::Private(_) |
                Attribute::Protected(_) |
                Attribute::Internal(_) => {
                    self.verify_visibility(&a, &context);
                },
                _ => {
                    // Unallowed attribute
                    self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
                },
            }
        }

        let node = Rc::new(Directive::FunctionDefinition(FunctionDefinition {
            location: self.pop_location(),
            asdoc,
            attributes,
            name: name.clone(),
            common,
        }));

        (node, semicolon)
    }

    fn parse_class_definition(&mut self, context: AnnotatableContext) -> (Rc<Directive>, bool) {
        let AnnotatableContext { start_location, asdoc, attributes, context, .. } = context;
        self.push_location(&start_location);
        let name = self.expect_identifier(true);
        let type_parameters = self.parse_type_parameters_opt();
        let mut extends_clause: Option<Rc<Expression>> = None;
        if self.consume(Token::Extends) {
            extends_clause = Some(self.parse_type_expression());
        }
        let mut implements_clause: Option<Vec<Rc<Expression>>> = None;
        if self.consume(Token::Implements) {
            implements_clause = Some(self.parse_type_expression_list());
        }
        let block = Rc::new(self.parse_block(ParserDirectiveContext::ClassBlock {
            name: name.0.clone(),
        }));

        for a in &attributes {
            if a.is_metadata() {
                continue;
            }
            match a {
                Attribute::Static(_) => {},
                Attribute::Final(_) => {},
                Attribute::Dynamic(_) => {},
                Attribute::Abstract(_) => {},

                Attribute::Expression(_) |
                Attribute::Public(_) |
                Attribute::Private(_) |
                Attribute::Protected(_) |
                Attribute::Internal(_) => {
                    self.verify_visibility(&a, &context);
                },
                _ => {
                    // Unallowed attribute
                    self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
                },
            }
        }

        // Nested classes not allowed
        if !context.is_top_level_or_package() {
            self.add_syntax_error(&name.1, DiagnosticKind::NestedClassesNotAllowed, diagarg![]);
        }

        let node = Rc::new(Directive::ClassDefinition(ClassDefinition {
            location: self.pop_location(),
            asdoc,
            attributes,
            name: name.clone(),
            type_parameters,
            extends_clause,
            implements_clause,
            block,
        }));

        (node, true)
    }

    fn parse_enum_definition(&mut self, context: AnnotatableContext) -> (Rc<Directive>, bool) {
        let AnnotatableContext { start_location, asdoc, mut attributes, context, .. } = context;
        self.push_location(&start_location);
        let name = self.expect_identifier(true);
        let mut as_clause: Option<Rc<Expression>> = None;
        if self.consume(Token::As) {
            as_clause = Some(self.parse_type_expression());
        }
        let block = Rc::new(self.parse_block(ParserDirectiveContext::EnumBlock));

        for a in &attributes {
            if a.is_metadata() {
                continue;
            }
            match a {
                Attribute::Expression(_) |
                Attribute::Public(_) |
                Attribute::Private(_) |
                Attribute::Protected(_) |
                Attribute::Internal(_) => {
                    self.verify_visibility(&a, &context);
                },
                _ => {
                    // Unallowed attribute
                    self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
                },
            }
        }

        // Nested classes not allowed
        if !context.is_top_level_or_package() {
            self.add_syntax_error(&name.1, DiagnosticKind::NestedClassesNotAllowed, diagarg![]);
        }

        let mut is_set = false;
        let metadata = Attribute::find_metadata(&attributes);
        for metadata in metadata {
            if metadata.name.0 == "Set" {
                is_set = true;
                Attribute::remove_metadata(&mut attributes, &metadata);
            }
        }

        let node = Rc::new(Directive::EnumDefinition(EnumDefinition {
            location: self.pop_location(),
            asdoc,
            attributes,
            is_set,
            name: name.clone(),
            as_clause,
            block,
        }));

        (node, true)
    }

    fn parse_interface_definition(&mut self, context: AnnotatableContext) -> (Rc<Directive>, bool) {
        let AnnotatableContext { start_location, asdoc, attributes, context, .. } = context;
        self.push_location(&start_location);
        let name = self.expect_identifier(true);
        let type_parameters = self.parse_type_parameters_opt();
        let mut extends_clause: Option<Vec<Rc<Expression>>> = None;
        if self.consume(Token::Extends) {
            extends_clause = Some(self.parse_type_expression_list());
        }
        let block = Rc::new(self.parse_block(ParserDirectiveContext::InterfaceBlock));

        // Interface block must only contain function definitions
        for directive in block.directives.iter() {
            if !(matches!(directive.as_ref(), Directive::FunctionDefinition(_))) {
                self.add_syntax_error(&directive.location(), DiagnosticKind::UnexpectedDirective, diagarg![]);
            }
        }

        for a in &attributes {
            if a.is_metadata() {
                continue;
            }
            match a {
                Attribute::Expression(_) |
                Attribute::Public(_) |
                Attribute::Private(_) |
                Attribute::Protected(_) |
                Attribute::Internal(_) => {
                    self.verify_visibility(&a, &context);
                },
                _ => {
                    // Unallowed attribute
                    self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
                },
            }
        }

        // Nested classes not allowed
        if !context.is_top_level_or_package() {
            self.add_syntax_error(&name.1, DiagnosticKind::NestedClassesNotAllowed, diagarg![]);
        }

        let node = Rc::new(Directive::InterfaceDefinition(InterfaceDefinition {
            location: self.pop_location(),
            asdoc,
            attributes,
            name: name.clone(),
            type_parameters,
            extends_clause,
            block,
        }));

        (node, true)
    }

    fn parse_type_definition(&mut self, context: AnnotatableContext) -> (Rc<Directive>, bool) {
        let AnnotatableContext { start_location, asdoc, attributes, context, .. } = context;
        self.push_location(&start_location);
        let left = self.expect_identifier(true);
        let mut right = self.create_invalidated_expression(&self.tokenizer.cursor_location());
        self.non_greedy_expect(Token::Assign);
        if !self.expecting_token_error {
            right = self.parse_type_expression();
        }

        for a in &attributes {
            if a.is_metadata() {
                continue;
            }
            match a {
                Attribute::Expression(_) |
                Attribute::Public(_) |
                Attribute::Private(_) |
                Attribute::Protected(_) |
                Attribute::Internal(_) => {
                    self.verify_visibility(&a, &context);
                },
                _ => {
                    // Unallowed attribute
                    self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
                },
            }
        }

        // Nested classes not allowed
        if !context.is_top_level_or_package() {
            self.add_syntax_error(&left.1, DiagnosticKind::NestedClassesNotAllowed, diagarg![]);
        }

        let semicolon = self.parse_semicolon();

        let node = Rc::new(Directive::TypeDefinition(TypeDefinition {
            location: self.pop_location(),
            asdoc,
            attributes,
            left: left.clone(),
            right,
        }));

        (node, semicolon)
    }

    fn parse_namespace_definition(&mut self, context: AnnotatableContext) -> (Rc<Directive>, bool) {
        let AnnotatableContext { start_location, asdoc, attributes, context, .. } = context;
        self.push_location(&start_location);
        let left = self.expect_identifier(true);
        let mut right: Option<Rc<Expression>> = None;
        if self.consume(Token::Assign) {
            right = Some(self.parse_expression(ParserExpressionContext {
                min_precedence: OperatorPrecedence::AssignmentAndOther,
                ..default()
            }));
        }

        for a in &attributes {
            if a.is_metadata() {
                continue;
            }
            match a {
                Attribute::Expression(_) |
                Attribute::Public(_) |
                Attribute::Private(_) |
                Attribute::Protected(_) |
                Attribute::Internal(_) => {
                    self.verify_visibility(&a, &context);
                },
                Attribute::Static(_) => {},
                _ => {
                    // Unallowed attribute
                    self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
                },
            }
        }

        let semicolon = self.parse_semicolon();

        let node = Rc::new(Directive::NamespaceDefinition(NamespaceDefinition {
            location: self.pop_location(),
            asdoc,
            attributes,
            left: left.clone(),
            right,
        }));

        (node, semicolon)
    }

    fn parse_type_expression_list(&mut self) -> Vec<Rc<Expression>> {
        let mut list = vec![self.parse_type_expression()];
        while self.consume(Token::Comma) {
            list.push(self.parse_type_expression());
        }
        list
    }

    fn verify_visibility(&self, a: &Attribute, context: &ParserDirectiveContext) {
        let mut unallowed = false;
        match a {
            Attribute::Expression(_) => {},
            Attribute::Public(_) => {},
            Attribute::Private(_) |
            Attribute::Protected(_) => {
                if !context.is_type_block() {
                    unallowed = true;
                }
            },
            Attribute::Internal(_) => {},
            _ => {}
        }
        if unallowed {
            // Unallowed attribute
            self.add_syntax_error(&a.location(), DiagnosticKind::UnallowedAttribute, diagarg![]);
        }
    }
    
    fn parse_type_parameters_opt(&mut self) -> Option<Vec<Rc<TypeParameter>>> {
        if !self.consume(Token::Dot) {
            return None;
        }
        let mut list: Vec<Rc<TypeParameter>> = vec![];
        self.non_greedy_expect(Token::Lt);
        if !self.expecting_token_error {
            list.push(self.parse_type_parameter());
            while self.consume(Token::Comma) {
                list.push(self.parse_type_parameter());
            }
            self.non_greedy_expect_type_parameters_gt();
        }
        Some(list)
    }
    
    fn parse_type_parameter(&mut self) -> Rc<TypeParameter> {
        self.mark_location();
        let name = self.expect_identifier(false);
        Rc::new(TypeParameter {
            location: self.pop_location(),
            name,
        })
    }

    fn keyword_or_expression_attribute_from_expression(&self, expr: &Rc<Expression>) -> Attribute {
        match expr.as_ref() {
            Expression::QualifiedIdentifier(id) => {
                if id.qualifier.is_some() || id.attribute {
                    return Attribute::Expression(expr.clone());
                }
                match &id.id {
                    QualifiedIdentifierIdentifier::Id((id, location)) => {
                        if let Some(attr) = Attribute::from_identifier_name(&id, &location) {
                            return attr;
                        }
                        Attribute::Expression(expr.clone())
                    },
                    _ => Attribute::Expression(expr.clone()),
                }
            },
            _ => Attribute::Expression(expr.clone()),
        }
    }

    fn keyword_attribute_from_previous_token(&self) -> Option<Attribute> {
        self.previous_token.0.to_attribute(&self.previous_token.1)
    }

    fn _keyword_or_expression_attribute_from_previous_token(&mut self) -> Option<Attribute> {
        if let Some(a) = self.keyword_attribute_from_previous_token() {
            return Some(a);
        }
        match &self.previous_token.0 {
            Token::Identifier(id) => Some(Attribute::Expression(self.parse_expression_attribute(&(id.clone(), self.previous_token.1.clone())))),
            _ => None,
        }
    }

    fn parse_keyword_or_expression_attribute(&mut self) -> Option<Attribute> {
        if let Some(a) = self.token.0.to_attribute(&self.token.1) {
            self.next();
            return Some(a);
        }
        match &self.token.0 {
            Token::Identifier(_) => {
                let id = self.expect_identifier(false);
                Some(Attribute::Expression(self.parse_expression_attribute(&id)))
            },
            _ => None,
        }
    }

    fn peek_annotatable_directive_identifier_name(&self) -> bool {
        if self.token.0.to_attribute(&self.token.1).is_some() {
            return true;
        }
        match self.token.0 {
            Token::Identifier(_) => true,
            Token::Var |
            Token::Const |
            Token::Function |
            Token::Class |
            Token::Interface => true,
            _ => false,
        }
    }

    fn lookbehind_is_annotatable_directive_identifier_name(&self) -> bool {
        self.keyword_attribute_from_previous_token().is_some()
        || matches!(&self.previous_token.0, Token::Identifier(_))
        || Token::is_context_keyword(&self.previous_token, "enum")
        || Token::is_context_keyword(&self.previous_token, "type")
        || Token::is_context_keyword(&self.previous_token, "namespace")
    }

    fn parse_attribute_keywords_or_expressions(&mut self, context: &mut AnnotatableContext) {
        if context.directive_context_keyword.is_some() {
            unreachable!();
        }
        loop {
            if let Some(a) = self.parse_keyword_or_expression_attribute() {
                if let Attribute::Expression(e) = &a {
                    let id = e.to_identifier_name();
                    if let Some(id) = id {
                        if ["enum", "type", "namespace"].contains(&id.0.as_ref()) {
                            context.directive_context_keyword = Some(id);
                            break;
                        }
                    }
                }
                let last_attribute_is_identifier = context.attributes.last().map_or(false, |a| !a.is_metadata());
                if last_attribute_is_identifier {
                    self.forbid_line_break_before_token();
                }
                context.attributes.push(a);
                // self.next();
            } else {
                if let Some(id) = self.peek_identifier(false) {
                    self.forbid_line_break_before_token();
                    if ["enum", "type", "namespace"].contains(&id.0.as_ref()) {
                        self.next();
                        context.directive_context_keyword = Some(id);
                    }
                }
                break;
            }
        }
        // For meta-data that are not one of certain Flex meta-data,
        // delegate the respective ASDoc to the annotatable directive.
        let mut new_attributes = Vec::<Attribute>::new();
        for attr in &context.attributes {
            if let Attribute::Metadata(metadata) = attr {
                if !self.documentable_metadata.contains(&metadata.name.0) && metadata.asdoc.is_some() {
                    new_attributes.push(Attribute::Metadata(Rc::new(Metadata {
                        location: metadata.location.clone(),
                        asdoc: None,
                        name: metadata.name.clone(),
                        entries: metadata.entries.clone(),
                    })));
                    context.asdoc = metadata.asdoc.clone();
                } else {
                    new_attributes.push(attr.clone());
                }
            } else {
                new_attributes.push(attr.clone());
            }
        }
        context.attributes = new_attributes;
    }

    pub fn parse_package_definition(&mut self) -> Rc<PackageDefinition> {
        self.mark_location();
        let asdoc = self.parse_asdoc();
        self.non_greedy_expect(Token::Package);
        let mut name = vec![];
        if let Some(name1) = self.consume_identifier(false) {
            name.push(name1.clone());
            while self.consume(Token::Dot) {
                name.push(self.expect_identifier(true));
            }
        }
        let block = Rc::new(self.parse_block(ParserDirectiveContext::PackageBlock));
        Rc::new(PackageDefinition {
            location: self.pop_location(),
            asdoc,
            name,
            block,
        })
    }

    pub fn parse_program(&mut self) -> Rc<Program> {
        self.mark_location();
        let just_eof = self.peek(Token::Eof);
        let mut packages = vec![];
        while self.peek(Token::Package) {
            packages.push(self.parse_package_definition());
        }
        let directives = self.parse_directives(ParserDirectiveContext::TopLevel);
        Rc::new(Program {
            location: if just_eof {
                self.pop_location();
                self.token.1.clone()
            } else {
                self.pop_location()
            },
            packages,
            directives,
        })
    }

    pub fn parse_asdoc(&mut self) -> Option<Rc<Asdoc>> {
        let comments = self.compilation_unit().comments.borrow();
        let last_comment = comments.last().map(|last_comment| last_comment.clone());
        drop(comments);
        last_comment.and_then(|comment| {
            if comment.is_asdoc(&self.token.1) {
                self.compilation_unit().comments_mut().pop();
                let location = comment.location();
                let comment_prefix_length: usize = 3;
                let location1 = Location::with_offsets(self.compilation_unit(), location.first_offset + comment_prefix_length, location.last_offset - 2);
                let content = &comment.content.borrow()[1..];
                let (main_body, tags) = self.parse_asdoc_content(&location1, content);
                Some(Rc::new(Asdoc {
                    location,
                    main_body,
                    tags,
                }))
            } else {
                None
            }
        })
    }

    fn parse_asdoc_content(&mut self, location: &Location, content: &str) -> (Option<(String, Location)>, Vec<(AsdocTag, Location)>) {
        let lines = self.split_asdoc_lines(location, content);

        let mut main_body: Option<(String, Location)> = None;
        let mut tags: Vec<(AsdocTag, Location)> = vec![];
        let mut i = 0;
        let line_count = lines.len();

        let mut building_content_tag_name: Option<(String, Location)> = None;
        let mut building_content: Vec<(String, Location)> = vec![];
        let mut inside_code_block = false;

        while i < line_count {
            let line = &lines[i];
            let tag = if inside_code_block { None } else {
                regex_captures!(r"^([\s\t]*\@)([^\s\t]+)(.*)", &line.content)
            };
            if let Some((_, tag_prefix, tag_name, tag_content)) = tag {
                self.parse_asdoc_tag_or_main_body(
                    &mut building_content_tag_name,
                    &mut building_content,
                    &mut main_body,
                    &mut tags,
                );
                if regex_is_match!(r"^[\s\t]*```([^`]|$)", &tag_content) {
                    inside_code_block = true;
                }
                let tag_name_location = Location::with_offsets(self.compilation_unit(), line.location.first_offset() + tag_prefix.len() - 1, line.location.first_offset() + tag_prefix.len() + tag_name.len());
                building_content_tag_name = Some((tag_name.into(), tag_name_location));
                let tag_content_location = Location::with_offsets(self.compilation_unit(), line.location.first_offset() + tag_prefix.len() + tag_name.len(), line.location.last_offset());
                building_content.push((tag_content.into(), tag_content_location));

                if ["private", "inheritDoc"].contains(&tag_name) {
                    self.parse_asdoc_tag_or_main_body(
                        &mut building_content_tag_name,
                        &mut building_content,
                        &mut main_body,
                        &mut tags,
                    );
                    building_content_tag_name = None;
                    building_content.clear();
                }
            } else {
                if regex_is_match!(r"^[\s\t]*```([^`]|$)", &line.content) {
                    inside_code_block = !inside_code_block;
                }
                building_content.push((line.content.clone(), line.location.clone()));
            }
            i += 1;
        }

        self.parse_asdoc_tag_or_main_body(
            &mut building_content_tag_name,
            &mut building_content,
            &mut main_body,
            &mut tags,
        );

        (main_body, tags)
    }

    fn split_asdoc_lines(&mut self, location: &Location, content: &str) -> Vec<ParserAsdocLine> {
        let mut builder = String::new();
        let mut lines = vec![];
        let mut _line_number = location.first_line_number();
        let mut index = location.first_offset();
        let mut line_first_offset = index;
        let mut characters = content.chars();
        while let Some(ch) = characters.next() {
            if CharacterValidator::is_line_terminator(ch) {
                lines.push(ParserAsdocLine {
                    content: builder,
                    location: Location::with_offsets(self.compilation_unit(), line_first_offset, index),
                });
                index += ch.len_utf8();
                // <CR><LF> sequence
                if ch == '\r' && characters.clone().next().unwrap_or('\x00') == '\n' {
                    index += '\n'.len_utf8();
                    characters.next();
                }
                builder = String::new();
                _line_number += 1;
                line_first_offset = index;
            } else {
                builder.push(ch);
                index += ch.len_utf8();
            }
        }
        lines.push(ParserAsdocLine {
            content: builder,
            location: Location::with_offsets(self.compilation_unit(), line_first_offset, index),
        });
        for line in &mut lines {
            let line_content = line.content.to_owned();
            let prefix = regex_captures!(r"^\s*(\*\s?)", &line_content);
            if let Some((prefix, _)) = prefix {
                line.content = line.content[prefix.len()..].to_owned();
                line.location = Location::with_offsets(self.compilation_unit(), line.location.first_offset() + prefix.len(), line.location.last_offset());
            }
        }

        lines
    }

    fn parse_asdoc_tag_or_main_body(
        &self,
        building_content_tag_name: &mut Option<(String, Location)>,
        building_content: &mut Vec<(String, Location)>,
        main_body: &mut Option<(String, Location)>,
        tags: &mut Vec<(AsdocTag, Location)>
    ) {
        if let Some((tag_name, ref tag_location)) = building_content_tag_name.as_ref() {
            match tag_name.as_ref() {
                // @author Author text
                "author" => {
                    let (content, location) = join_asdoc_content(building_content);
                    // Content must be non empty
                    if regex_is_match!(r"^\s*$", &content) {
                        self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.clone()]);
                    }
                    let location = tag_location.combine_with(location);
                    tags.push((AsdocTag::Author(content), location));
                },

                // @copy reference
                "copy" => {
                    let (content, c_location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(c_location.clone());
                    let reference_loc = c_location.shift_whitespace(&self.compilation_unit().text()[c_location.first_offset()..c_location.last_offset()]);
                    if let Some(reference) = self.parse_asdoc_reference(&content, &reference_loc, &tag_location, &tag_name) {
                        tags.push((AsdocTag::Copy(reference), location));
                    }
                },

                // @created Date text
                "created" => {
                    let (content, location) = join_asdoc_content(building_content);
                    // Content must be non empty
                    if regex_is_match!(r"^\s*$", &content) {
                        self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.clone()]);
                    }
                    let location = tag_location.combine_with(location);
                    tags.push((AsdocTag::Created(content), location));
                },

                // @default value
                "default" => {
                    let (reference, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);
                    tags.push((AsdocTag::Default(reference), location));
                },

                // @deprecated
                "deprecated" => {
                    let (text, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);

                    let mut message: Option<String> = None;

                    if !regex_is_match!(r"^\s*$", &text) {
                        message = Some(text.clone());
                    }

                    tags.push((AsdocTag::Deprecated { message }, location));
                },

                // @eventType typeOrConstant
                "eventType" => {
                    let (_, c_location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(c_location.clone());
                    let reference_loc = c_location.shift_whitespace(&self.compilation_unit().text()[c_location.first_offset()..c_location.last_offset()]);
                    let parser_options = ParserOptions {
                        byte_range: Some((reference_loc.first_offset(), reference_loc.last_offset())),
                        ..self.options()
                    };
                    let exp = ParserFacade(self.compilation_unit(), parser_options).parse_expression();
                    tags.push((AsdocTag::EventType(exp), location));
                },

                // @example text
                "example" => {
                    let (text, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);
                    tags.push((AsdocTag::Example(text), location));
                },

                // @inheritDoc
                "inheritDoc" => {
                    let (text, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);

                    // Content must be empty
                    if !regex_is_match!(r"^\s*$", &text) {
                        self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.clone()]);
                    }

                    tags.push((AsdocTag::InheritDoc, location));
                },

                // @internal text
                "internal" => {
                    let (text, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);

                    // Content must be non empty
                    if regex_is_match!(r"^\s*$", &text) {
                        self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.clone()]);
                    }

                    tags.push((AsdocTag::Internal(text), location));
                },

                // @langversion text
                "langversion" => {
                    let (text, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);

                    // Content must be non empty
                    if regex_is_match!(r"^\s*$", &text) {
                        self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.clone()]);
                    }

                    tags.push((AsdocTag::Langversion(text), location));
                },

                // @param paramName description
                "param" => {
                    let (content, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);

                    if let Some((_, name, description)) = regex_captures!(r"(?x) ([^\s]+) (.*)", &content) {
                        tags.push((AsdocTag::Param { name: name.into(), description: description.trim_start().into() }, location));
                    } else {
                        tags.push((AsdocTag::Param { name: content, description: "".into() }, location));
                    }
                },

                // @playerversion text
                "playerversion" => {
                    let (text, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);

                    // Content must be non empty
                    if regex_is_match!(r"^\s*$", &text) {
                        self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.clone()]);
                    }

                    tags.push((AsdocTag::Playerversion(text), location));
                },

                // @private
                "private" => {
                    let (text, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);

                    // Content must be empty
                    if !regex_is_match!(r"^\s*$", &text) {
                        self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.clone()]);
                    }

                    tags.push((AsdocTag::Private, location));
                },

                // @productversion text
                "productversion" => {
                    let (text, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);

                    // Content must be non empty
                    if regex_is_match!(r"^\s*$", &text) {
                        self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.clone()]);
                    }

                    tags.push((AsdocTag::Productversion(text), location));
                },

                // @return text
                "return" => {
                    let (text, location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(location);
                    tags.push((AsdocTag::Return(text), location));
                },

                // @see reference [displayText]
                "see" => {
                    let (content, c_location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(c_location.clone());
                    let reference: String;
                    let display_text: Option<String>;
                    let mut reference_loc = c_location.shift_whitespace(&self.compilation_unit().text()[c_location.first_offset()..c_location.last_offset()]);
                    if let Some((_, reference_1, display_text_1)) = regex_captures!(r"(?x) ([^\s]+) (.*)", &content) {
                        reference = reference_1.to_owned();
                        reference_loc = Location::with_offsets(self.compilation_unit(), reference_loc.first_offset(), reference_loc.first_offset() + reference.len());
                        display_text = Some(display_text_1.trim().to_owned());
                    } else {
                        reference = content;
                        display_text = None;
                    }
                    if let Some(reference) = self.parse_asdoc_reference(&reference, &reference_loc, &tag_location, &tag_name) {
                        tags.push((AsdocTag::See { reference, display_text }, location));
                    }
                },

                // @throws className description
                "throws" => {
                    let (class_name_and_description, c_location) = join_asdoc_content(building_content);
                    let location = tag_location.combine_with(c_location.clone());

                    let class_name_and_description = regex_captures!(r"^([^\s]+)(\s.*)?", &class_name_and_description);

                    if let Some((_, class_name, description)) = class_name_and_description {
                        let description = description.trim().to_owned();
                        let description = if description.is_empty() {
                            None
                        } else {
                            Some(description)
                        };
                        let mut reference_loc = c_location.shift_whitespace(&self.compilation_unit().text()[c_location.first_offset()..c_location.last_offset()]);
                        reference_loc = Location::with_offsets(self.compilation_unit(), reference_loc.first_offset(), reference_loc.first_offset() + class_name.len());
                        let parser_options = ParserOptions {
                            byte_range: Some((reference_loc.first_offset(), reference_loc.last_offset())),
                            ..self.options()
                        };
                        let exp = ParserFacade(self.compilation_unit(), parser_options).parse_type_expression();
                        tags.push((AsdocTag::Throws { class_reference: exp, description }, location));
                    } else {
                        self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.clone()]);
                    }
                },

                // @version Version text
                "version" => {
                    let (content, location) = join_asdoc_content(building_content);
                    // Content must be non empty
                    if regex_is_match!(r"^\s*$", &content) {
                        self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.clone()]);
                    }
                    let location = tag_location.combine_with(location);
                    tags.push((AsdocTag::Version(content), location));
                },

                // Unrecognized tag
                _ => {
                    self.add_syntax_error(&tag_location, DiagnosticKind::UnrecognizedAsdocTag, diagarg![tag_name.clone()]);
                },
            }
        } else if !building_content.is_empty() {
            let content = join_asdoc_content(building_content);
            if !content.0.is_empty() {
                *main_body = Some(content);
            }
        }

        *building_content_tag_name = None;
        building_content.clear();
    }

    fn parse_asdoc_reference(&self, reference: &str, reference_loc: &Location, tag_location: &Location, tag_name: &str) -> Option<Rc<AsdocReference>> {
        let split: Vec<&str> = reference.split("#").collect();
        if split.len() > 2 {
            self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.to_owned()]);
            return None;
        }
        let mut base: Option<Rc<Expression>> = None;
        let base_text: String = split[0].to_owned();
        let instance_property_text: Option<(String, Location)> = split.get(1).and_then(|&f| if f.is_empty() { None } else {
            Some((f.to_owned(), Location::with_offsets(self.compilation_unit(), reference_loc.first_offset() + base_text.len() + 1, reference_loc.last_offset())))
        });

        if !base_text.is_empty() {
            let parser_options = ParserOptions {
                byte_range: Some((reference_loc.first_offset(), reference_loc.first_offset() + base_text.len())),
                ..self.options()
            };
            let exp = ParserFacade(self.compilation_unit(), parser_options).parse_expression();
            base = Some(exp);
        }

        let mut instance_property: Option<Rc<QualifiedIdentifier>> = None;
        if let Some(text) = instance_property_text {
            let parser_options = ParserOptions {
                byte_range: Some((text.1.first_offset(), text.1.last_offset())),
                ..self.options()
            };
            let exp = ParserFacade(self.compilation_unit(), parser_options).parse_qualified_identifier();
            instance_property = Some(Rc::new(exp));
        }

        if base.is_none() && instance_property.is_none() {
            self.add_syntax_error(&tag_location, DiagnosticKind::FailedParsingAsdocTag, diagarg![tag_name.to_owned()]);
            return None;
        }
        Some(Rc::new(AsdocReference { base, instance_property, }))
    }

    /// Parses MXMLElement starting from its XMLTagContent.
    fn parse_mxml_element(&mut self, start: Location, namespace: &Rc<MxmlNamespace>, encoding: &mut String) -> MxmlElement {
        self.push_location(&start);
        let namespace = Rc::new(MxmlNamespace::new(Some(namespace)));
        let name = self.parse_xml_name();
        let mut attributes: Vec<Rc<MxmlAttribute>> = vec![];
        let mut plain_attributes: Vec<PlainMxmlAttribute> = vec![];
        while self.consume_and_ie_xml_tag(Token::XmlWhitespace) {
            if matches!(self.token.0, Token::XmlName(_)) {
                self.mark_location();
                let name = self.parse_xml_name();
                self.consume_and_ie_xml_tag(Token::XmlWhitespace);
                self.non_greedy_expect_and_ie_xml_tag(Token::Assign);
                let mut value = ("".into(), self.token.1.clone());
                if !self.expecting_token_error {
                    self.consume_and_ie_xml_tag(Token::XmlWhitespace);
                    value = self.parse_xml_attribute_value();
                }
                let attrib = PlainMxmlAttribute {
                    location: self.pop_location(),
                    name,
                    value,
                };
                self.process_mxml_xmlns_attribute(&mut attributes, &attrib, &namespace);
                plain_attributes.push(attrib);
            } else {
                break;
            }
        }

        for attrib in &plain_attributes {
            self.process_mxml_attribute(&mut attributes, &attrib, &namespace);
        }

        let name = self.process_mxml_tag_name(name, &namespace);

        let mut content: Option<Vec<Rc<MxmlContent>>> = None;
        let mut closing_name: Option<MxmlTagName> = None;

        let is_empty = self.consume_and_ie_xml_content(Token::XmlSlashGt);

        if !is_empty {
            self.expect_and_ie_xml_content(Token::Gt);
            content = Some(self.parse_mxml_content(false, &namespace, encoding));
            self.non_greedy_expect_and_ie_xml_tag(Token::XmlLtSlash);
            let name_1 = self.parse_xml_name();
            let closing_name_1 = self.process_mxml_tag_name(name_1, &namespace);
            if let Ok(equal) = name.equals_name(&closing_name_1, &namespace) {
                if !equal {
                    self.add_syntax_error(&closing_name_1.location, DiagnosticKind::XmlClosingTagNameMustBeEquals, diagarg![name.to_string(&namespace)]);
                }
            }
            closing_name = Some(closing_name_1);
            self.consume_and_ie_xml_tag(Token::XmlWhitespace);
            self.non_greedy_expect_and_ie_xml_content(Token::Gt);
        }

        if let Some(content) = content.as_mut() {
            self.filter_mxml_whitespace_out(content);
        }

        MxmlElement {
            location: self.pop_location(),
            name,
            attributes,
            content,
            closing_name,
            namespace,
        }
    }

    /// Filters whitespace chunks out of a content list when
    /// they include at least one child element.
    fn filter_mxml_whitespace_out(&self, content: &mut Vec<Rc<MxmlContent>>) {
        if !self.ignore_xml_whitespace {
            return;
        }
        let mut inc_el = false;
        for node in content.iter() {
            inc_el = matches!(node.as_ref(), MxmlContent::Element(_));
            if inc_el {
                break;
            }
        }
        if inc_el {
            let mut indices: Vec<usize> = vec![];
            for i in 0..content.len() {
                let MxmlContent::Characters((ch, _)) = content[i].as_ref() else {
                    continue;
                };
                if ch.trim().is_empty() {
                    indices.push(i);
                }
            }
            for i in indices.iter().rev() {
                content.remove(*i);
            }
        }
    }

    fn process_mxml_xmlns_attribute(&mut self, output: &mut Vec<Rc<MxmlAttribute>>, attribute: &PlainMxmlAttribute, namespace: &Rc<MxmlNamespace>) {
        // xml="uri"
        if attribute.name.0 == "xmlns" {
            let attribute_value = unescape_xml(&attribute.value.0);
            namespace.set(MxmlNamespace::DEFAULT_NAMESPACE, &attribute_value);
            output.push(Rc::new(MxmlAttribute {
                location: attribute.location.clone(),
                name: MxmlName {
                    location: attribute.name.1.clone(),
                    prefix: None,
                    name: "xmlns".into(),
                },
                value: (attribute_value, attribute.value.1.clone()),
                xmlns: true,
            }));
        // xmlns:prefix="uri"
        } else if attribute.name.0.starts_with("xmlns:") {
            let attribute_value = unescape_xml(&attribute.value.0);
            namespace.set(&attribute.name.0[6..], &attribute_value);
            if attribute.name.0[6..].find(':').is_some() {
                self.add_syntax_error(&attribute.name.1, DiagnosticKind::XmlNameAtMostOneColon, vec![]);
            }
            output.push(Rc::new(MxmlAttribute {
                location: attribute.location.clone(),
                name: MxmlName {
                    location: attribute.name.1.clone(),
                    prefix: Some("xmlns".into()),
                    name: attribute.name.0[6..].to_owned(),
                },
                value: (attribute_value, attribute.value.1.clone()),
                xmlns: true,
            }));
        }
    }

    fn process_mxml_attribute(&mut self, output: &mut Vec<Rc<MxmlAttribute>>, attribute: &PlainMxmlAttribute, namespace: &Rc<MxmlNamespace>) {
        // attrib="value"
        if !(attribute.name.0 == "xmlns" || attribute.name.0.starts_with("xmlns:")) {
            let attribute_value = unescape_xml(&attribute.value.0);
            let split = attribute.name.0.split(':').collect::<Vec<_>>();
            if split.len() > 2 {
                self.add_syntax_error(&attribute.name.1, DiagnosticKind::XmlNameAtMostOneColon, vec![]);
            }
            let prefix: Option<String> = if split.len() > 1 {
                Some(split[split.len() - 2].to_owned())
            } else {
                None
            };
            let name = split.last().unwrap();
            let attrib = Rc::new(MxmlAttribute {
                location: attribute.location.clone(),
                name: MxmlName {
                    location: attribute.name.1.clone(),
                    prefix,
                    name: (*name).to_owned(),
                },
                value: (attribute_value, attribute.value.1.clone()),
                xmlns: false,
            });
            match attrib.name.resolve_prefix(namespace) {
                Ok(_) => {
                    for prev_attrib in output.iter() {
                        if prev_attrib.name.equals_name(&attrib.name, namespace).unwrap_or(false) {
                            self.add_syntax_error(&attrib.name.location, DiagnosticKind::RedefiningXmlAttribute, diagarg![attrib.name.name.clone()]);
                        }
                    }
                },
                Err(MxmlNameError::PrefixNotDefined(prefix)) => {
                    self.add_syntax_error(&attrib.name.location, DiagnosticKind::XmlPrefixNotDefined, diagarg![prefix]);
                },
            }
            output.push(attrib);
        }
    }

    fn process_mxml_tag_name(&mut self, name: (String, Location), namespace: &Rc<MxmlNamespace>) -> MxmlTagName {
        let split = name.0.split(':').collect::<Vec<_>>();
        if split.len() > 2 {
            self.add_syntax_error(&name.1, DiagnosticKind::XmlNameAtMostOneColon, vec![]);
        }
        let prefix: Option<String> = if split.len() > 1 {
            Some(split[split.len() - 2].to_owned())
        } else {
            None
        };
        let name_str = split.last().unwrap();
        let name = MxmlTagName {
            location: name.1.clone(),
            prefix,
            name: (*name_str).to_owned(),
        };
        match name.resolve_prefix(namespace) {
            Ok(_) => {},
            Err(MxmlNameError::PrefixNotDefined(prefix)) => {
                self.add_syntax_error(&name.location, DiagnosticKind::XmlPrefixNotDefined, diagarg![prefix]);
            },
        }
        name
    }

    /// Parses XMLContent until either the `</` token or end-of-file.
    fn parse_mxml_content(&mut self, until_eof: bool, namespace: &Rc<MxmlNamespace>, encoding: &mut String) -> Vec<Rc<MxmlContent>> {
        let mut content = vec![];
        while if until_eof { self.tokenizer.characters().has_remaining() } else { !self.peek(Token::XmlLtSlash) } {
            if let Token::XmlMarkup(markup) = self.token.0.clone() {
                let location = self.token_location();
                self.next_ie_xml_content();
                // XMLCDATA
                if markup.starts_with("<![CDATA[") {
                    content.push(Rc::new(MxmlContent::CData((markup, location))));
                // XMLComment
                } else if markup.starts_with("<!--") {
                    content.push(Rc::new(MxmlContent::Comment((markup, location))));
                // XMLPI
                } else {
                    let mut pi_characters = CharacterReader::from(&markup[2..(markup.len() - 2)]);
                    let mut name = String::new();
                    if CharacterValidator::is_xml_name_start(pi_characters.peek_or_zero()) {
                        name.push(pi_characters.next_or_zero());
                        while CharacterValidator::is_xml_name_part(pi_characters.peek_or_zero()) {
                            name.push(pi_characters.next_or_zero());
                        }
                    }
                    let mut data = String::new();
                    while pi_characters.has_remaining() {
                        data.push(pi_characters.next_or_zero());
                    }

                    let i = location.first_offset() + 2 + name.len();
                    let j = decrease_last_offset(i, location.last_offset(), 2);

                    let errors = process_xml_pi(self.compilation_unit(), (i, j), &name, encoding);
                    for error in errors.iter() {
                        match error {
                            XmlPiError::UnknownAttribute(name) => {
                                self.add_syntax_error(&location, DiagnosticKind::XmlPiUnknownAttribute, diagarg![name.clone()]);
                            },
                            XmlPiError::Version => {
                                self.add_syntax_error(&location, DiagnosticKind::XmlPiVersion, vec![]);
                            },
                            XmlPiError::Encoding => {
                                self.add_syntax_error(&location, DiagnosticKind::XmlPiEncoding, vec![]);
                            },
                        }
                    }
                    content.push(Rc::new(MxmlContent::ProcessingInstruction {
                        location,
                        name,
                        data: if data.is_empty() { None } else { Some(data) },
                    }));
                }
            } else if let Token::XmlText(text) = self.token.0.clone() {
                let location = self.token_location();
                self.next_ie_xml_content();
                content.push(Rc::new(MxmlContent::Characters((unescape_xml(&text), location))));
            } else if self.consume_and_ie_xml_tag(Token::Lt) {
                let start = self.token_location();
                let element = self.parse_mxml_element(start, namespace, encoding);
                content.push(Rc::new(MxmlContent::Element(Rc::new(element))));
            } else if !until_eof {
                self.non_greedy_expect_and_ie_xml_content(Token::XmlLtSlash);
                if !self.tokenizer.characters().has_remaining() {
                    break;
                }
            } else if self.peek(Token::XmlLtSlash) {
                self.add_syntax_error(&self.token_location(), DiagnosticKind::Expecting, diagarg![Token::Eof, self.token.0.clone()]);
                self.next_ie_xml_tag();
                let _ = self.parse_xml_name();
                self.consume_and_ie_xml_tag(Token::XmlWhitespace);
                self.non_greedy_expect_and_ie_xml_content(Token::Gt);
            }
        }
        content
    }

    fn parse_mxml(&mut self) -> Rc<Mxml> {
        self.mark_location();
        let ns = Rc::new(MxmlNamespace::new(None));
        let mut encoding = "utf-8".to_owned();
        let mut content = self.parse_mxml_content(true, &ns, &mut encoding);
        self.filter_mxml_whitespace_out(&mut content);

        let mut element_count = 0usize;
        let mut character_count = 0usize;

        for node in content.iter() {
            match node.as_ref() {
                MxmlContent::Characters(_) |
                MxmlContent::CData(_) => {
                    character_count += 1;
                },
                MxmlContent::Element(_) => {
                    element_count += 1;
                },
                _ => {},
            }
        }
        let location = self.pop_location();
        if element_count != 1 || character_count != 0 {
            self.add_syntax_error(&location, DiagnosticKind::XmlMustConsistOfExactly1Element, vec![]);
        }
        Rc::new(Mxml {
            location,
            version: XmlVersion::Version10,
            encoding,
            content,
        })
    }
}

fn parse_include_directive_source(nested_compilation_unit: Rc<CompilationUnit>, context: ParserDirectiveContext) -> (Vec<Rc<PackageDefinition>>, Vec<Rc<Directive>>) {
    let mut parser = Parser::new(&nested_compilation_unit, &ParserOptions {
        ..default()
    });
    parser.next();
    let mut packages = vec![];
    if matches!(context, ParserDirectiveContext::TopLevel) {
        while parser.peek(Token::Package) {
            packages.push(parser.parse_package_definition());
        }
    }
    (packages, parser.parse_directives(context))
}

fn join_asdoc_content(content: &Vec<(String, Location)>) -> (String, Location) {
    // Ignore first empty lines
    let mut i = 0usize;
    for content1 in content.iter() {
        if content1.0.trim().is_empty() {
            i += 1;
        } else {
            break;
        }
    }

    // Ignore last empty lines
    let mut j = content.len();
    for content1 in content.iter().rev() {
        if content1.0.trim().is_empty() {
            j -= 1;
        } else {
            break;
        }
    }

    if i > j {
        i = j;
    }

    let s: Vec<String> = content[i..j].iter().map(|c| c.0.clone()).collect();
    let s = s.join("\n").trim().to_owned();
    let location = if i == j {
        content[i].1.clone()
    } else {
        content[i].1.combine_with(content[i..j].last().unwrap().1.clone())
    };
    (s, location)
}

fn process_xml_pi(cu: &Rc<CompilationUnit>, byte_range: (usize, usize), name: &str, encoding: &mut String) -> Vec<XmlPiError> {
    if name != "xml" {
        return vec![];
    }
    let mut parser = Parser::new(&cu, &ParserOptions {
        byte_range: Some(byte_range),
        ..default()
    });
    let mut errors = Vec::<XmlPiError>::new();
    parser.next_ie_xml_tag();
    while parser.consume_and_ie_xml_tag(Token::XmlWhitespace) {
        if matches!(parser.token.0, Token::XmlName(_)) {
            let name = parser.parse_xml_name();
            parser.consume_and_ie_xml_tag(Token::XmlWhitespace);
            parser.expect_and_ie_xml_tag(Token::Assign);
            parser.consume_and_ie_xml_tag(Token::XmlWhitespace);
            let value = parser.parse_xml_attribute_value();
            match name.0.as_ref() {
                "version" => {
                    if value.0 != "1.0" {
                        errors.push(XmlPiError::Version);
                    }
                },
                "encoding" => {
                    let v = value.0.to_lowercase();
                    if ["utf-8", "utf-16"].contains(&v.as_str()) {
                        *encoding = v;
                    } else {
                        errors.push(XmlPiError::Encoding);
                    }
                },
                _ => {
                    errors.push(XmlPiError::UnknownAttribute(name.0.clone()));
                },
            }
        } else {
            break;
        }
    }
    parser.expect_eof();
    errors
}

enum XmlPiError {
    UnknownAttribute(String),
    Version,
    Encoding,
}

struct ParserAsdocLine {
    content: String,
    location: Location,
}

#[derive(Clone)]
struct ParserActivation {
    uses_yield: bool,
    uses_await: bool,
}

impl ParserActivation {
    pub fn new() -> Self {
        Self {
            uses_yield: false,
            uses_await: false,
        }
    }
}

#[derive(Clone)]
struct AnnotatableContext {
    start_location: Location,
    asdoc: Option<Rc<Asdoc>>,
    attributes: Vec<Attribute>,
    context: ParserDirectiveContext,
    /// Previous token as a directive context keyword.
    directive_context_keyword: Option<(String, Location)>,
}

impl AnnotatableContext {
    pub fn has_directive_context_keyword(&self, name: &str) -> bool {
        if let Some((ref k, _)) = self.directive_context_keyword {
            k == name
        } else {
            false
        }
    }
}

struct PlainMxmlAttribute {
    pub location: Location,
    pub name: (String, Location),
    pub value: (String, Location),
}

/// A simplified interface for executing the parser.
pub struct ParserFacade<'input>(pub &'input Rc<CompilationUnit>, pub ParserOptions);

pub struct ParserOptions {
    /// For MXML, indicates whether to ignore XML whitespace chunks when at
    /// least one element appears. Default: true.
    pub ignore_xml_whitespace: bool,
    /// Indicates the range of characters that shall be parsed,
    /// the first and last byte indices respectively.
    pub byte_range: Option<(usize, usize)>,
    /// Indicates the set of meta-data that are documentable through ASDoc comments.
    /// Defaults to \[`Event`, `SkinState`\].
    pub documentable_metadata: Vec<String>,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            ignore_xml_whitespace: true,
            byte_range: None,
            documentable_metadata: vec!["Event".into(), "SkinState".into()],
        }
    }
}

impl<'input> ParserFacade<'input> {
    fn create_parser(&self) -> Parser<'input> {
        Parser::new(self.0, &self.1)
    }

    /// Parses `Program` until end-of-file.
    pub fn parse_program(&self) -> Rc<Program> {
        let mut parser = self.create_parser();
        parser.next();
        parser.parse_program()
    }

    /// Parses `ListExpression^allowIn` and expects end-of-file.
    pub fn parse_expression(&self) -> Rc<Expression> {
        let mut parser = self.create_parser();
        parser.next();
        let exp = parser.parse_expression(ParserExpressionContext {
            ..default()
        });
        parser.expect_eof();
        exp
    }

    /// Parses a qualified identifier and expects end-of-file.
    pub fn parse_qualified_identifier(&self) -> QualifiedIdentifier {
        let mut parser = self.create_parser();
        parser.next();
        let exp = parser.parse_qualified_identifier();
        parser.expect_eof();
        exp
    }

    /// Parses `TypeExpression` and expects end-of-file.
    pub fn parse_type_expression(&self) -> Rc<Expression> {
        let mut parser = self.create_parser();
        parser.next();
        let exp = parser.parse_type_expression();
        parser.expect_eof();
        exp
    }

    /// Parses `Directives` until end-of-file.
    pub fn parse_directives(&self, context: ParserDirectiveContext) -> Vec<Rc<Directive>> {
        let mut parser = self.create_parser();
        parser.next();
        parser.parse_directives(context)
    }

    /// Parses `Mxml` until end-of-file.
    pub fn parse_mxml(&self) -> Rc<Mxml> {
        let mut parser = self.create_parser();
        parser.next_ie_xml_content();
        parser.parse_mxml()
    }

    /// Parses a sequence of zero or meta data and an ASDoc comment.
    pub fn parse_metadata(&self) -> (Vec<Attribute>, Option<Rc<Asdoc>>) {
        let mut parser = self.create_parser();
        parser.next();
        parser.parse_metadata()
    }

    /// Parses the content inside the square brackets (`[ ... ]`) of a meta data.
    pub fn parse_metadata_content(&self) -> Rc<Metadata> {
        let mut parser = self.create_parser();
        parser.next();
        parser.parse_metadata_content()
    }
}
