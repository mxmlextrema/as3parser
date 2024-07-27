use lazy_static::lazy_static;
use maplit::hashmap;
use crate::ns::*;

lazy_static! {
    pub static ref DATA: HashMap<i32, String> = hashmap! {
        // DiagnosticKind::K.id() => ".".into(),
        DiagnosticKind::InvalidEscapeValue.id() => "Invalid escape value.".into(),
        DiagnosticKind::UnexpectedEnd.id() => "Unexpected end-of-file.".into(),
        DiagnosticKind::UnallowedNumericSuffix.id() => "Unallowed numeric suffix.".into(),
        DiagnosticKind::StringLiteralMustBeTerminatedBeforeLineBreak.id() => "A string literal must be terminated before the line break.".into(),
        DiagnosticKind::Expecting.id() => "Expecting {1} before {2}.".into(),
        DiagnosticKind::ExpectingIdentifier.id() => "Expecting identifier before {1}.".into(),
        DiagnosticKind::ExpectingExpression.id() => "Expecting expression before {1}.".into(),
        DiagnosticKind::ExpectingXmlName.id() => "Expecting XML name before {1}.".into(),
        DiagnosticKind::ExpectingXmlAttributeValue.id() => "Expecting XML attribute value before {1}.".into(),
        DiagnosticKind::IllegalNullishCoalescingLeftOperand.id() => "Illegal nullish coalescing left operand.".into(),
        DiagnosticKind::WrongParameterPosition.id() => "Wrong parameter position.".into(),
        DiagnosticKind::DuplicateRestParameter.id() => "Duplicate rest parameter.".into(),
        DiagnosticKind::NotAllowedHere.id() => "{1} not allowed here.".into(),
        DiagnosticKind::MalformedRestParameter.id() => "Malformed rest parameter.".into(),
        DiagnosticKind::IllegalForInInitializer.id() => "Illegal 'for..in' initializer.".into(),
        DiagnosticKind::MultipleForInBindings.id() => "Multiple 'for..in' bindings are not allowed.".into(),
        DiagnosticKind::UndefinedLabel.id() => "Undefined label '{1}'.".into(),
        DiagnosticKind::IllegalContinue.id() => "Illegal continue statement.".into(),
        DiagnosticKind::IllegalBreak.id() => "Illegal break statement.".into(),
        DiagnosticKind::ExpressionMustNotFollowLineBreak.id() => "Expression must not follow line break.".into(),
        DiagnosticKind::TokenMustNotFollowLineBreak.id() => "Token must not follow line break.".into(),
        DiagnosticKind::ExpectingStringLiteral.id() => "Expecting string literal before {1}.".into(),
        DiagnosticKind::DuplicateAttribute.id() => "Duplicate attribute.".into(),
        DiagnosticKind::DuplicateAccessModifier.id() => "Duplicate access modifier.".into(),
        DiagnosticKind::ExpectingDirectiveKeyword.id() => "Expecting either 'var', 'const', 'function', 'class' or 'interface'.".into(),
        DiagnosticKind::UnallowedAttribute.id() => "Unallowed attribute.".into(),
        DiagnosticKind::UseDirectiveMustContainPublic.id() => "Use directive must contain the 'public' attribute.".into(),
        DiagnosticKind::MalformedEnumMember.id() => "Malformed enumeration member.".into(),
        DiagnosticKind::FunctionMayNotBeGenerator.id() => "Function may not be generator.".into(),
        DiagnosticKind::FunctionMayNotBeAsynchronous.id() => "Function may not be asynchronous.".into(),
        DiagnosticKind::FunctionMustNotContainBody.id() => "Function must not contain body.".into(),
        DiagnosticKind::FunctionMustContainBody.id() => "Function must contain body.".into(),
        DiagnosticKind::FunctionMustNotContainAnnotations.id() => "Function must not contain annotations.".into(),
        DiagnosticKind::NestedClassesNotAllowed.id() => "Nested classes are not allowed.".into(),
        DiagnosticKind::UnexpectedDirective.id() => "Unexpected directive.".into(),
        DiagnosticKind::FailedParsingAsdocTag.id() => "Failed parsing contents of ASDoc tag: '@{1}'.".into(),
        DiagnosticKind::UnrecognizedAsdocTag.id() => "Unrecognized ASDoc tag: '@{1}'.".into(),
        DiagnosticKind::UnrecognizedProxy.id() => "Unrecognized proxy: '{1}'.".into(),
        DiagnosticKind::EnumMembersMustBeConst.id() => "Enumeration members must be 'const'.".into(),
        DiagnosticKind::UnrecognizedMetadataSyntax.id() => "Unrecognized meta-data syntax.".into(),
        DiagnosticKind::FailedToIncludeFile.id() => "Failed to include file.".into(),
        DiagnosticKind::ParentSourceIsNotAFile.id() => "Parent source is not a file.".into(),
        DiagnosticKind::CircularIncludeDirective.id() => "Circular include directive.".into(),
        DiagnosticKind::MalformedDestructuring.id() => "Malformed destructuring.".into(),
        DiagnosticKind::XmlPrefixNotDefined.id() => "Prefix not defined: '{1}'.".into(),
        DiagnosticKind::RedefiningXmlAttribute.id() => "Redefining attribute: '{1}'.".into(),
        DiagnosticKind::InvalidXmlPi.id() => "Invalid processing instruction.".into(),
        DiagnosticKind::XmlPiUnknownAttribute.id() => "Unknown attribute at processing instruction: '{1}'.".into(),
        DiagnosticKind::XmlPiVersion.id() => "XML version must be '1.0'.".into(),
        DiagnosticKind::XmlPiEncoding.id() => "XML encoding must be either 'utf-8' or 'utf-16'.".into(),
        DiagnosticKind::XmlMustConsistOfExactly1Element.id() => "Document must consist of exactly one element.".into(),
        DiagnosticKind::XmlNameAtMostOneColon.id() => "XML name may have at most one colon.".into(),
        DiagnosticKind::UnexpectedCharacter.id() => "Unexpected character. '{1}' is not allowed here".into(),
        DiagnosticKind::InputEndedBeforeReachingClosingQuoteForString.id() => "Input ended before reaching the closing quotation mark for a string literal.".into(),
        DiagnosticKind::InputEndedBeforeReachingClosingSeqForCData.id() => "Input ended before reaching the closing ']]>' for a CDATA.".into(),
        DiagnosticKind::InputEndedBeforeReachingClosingSeqForPi.id() => "Input ended before reaching the closing '?>' for a processing instruction.".into(),
        DiagnosticKind::InputEndedBeforeReachingClosingSeqForXmlComment.id() => "Input ended before reaching the closing '-->' for a comment.".into(),
        DiagnosticKind::InputEndedBeforeReachingClosingSeqForMultiLineComment.id() => "Input ended before reaching the closing '*/' for a comment.".into(),
        DiagnosticKind::InputEndedBeforeReachingClosingSlashForRegExp.id() => "Input ended before reaching the closing slash for a regular expression.".into(),
        DiagnosticKind::InputEndedBeforeReachingClosingQuoteForAttributeValue.id() => "Input ended before reaching the closing quotation mark for an attribute value.".into(),
        DiagnosticKind::ExpectingEitherSemicolonOrNewLineHere.id() => "Expecting either a semicolon or a new line here.".into(),
        DiagnosticKind::CssInvalidHexEscape.id() => "Invalid hexadecimal escape: '\\{1}'.".into(),
        DiagnosticKind::ExpectingDirective.id() => "Expecting directive before {1}.".into(),
        DiagnosticKind::ExpectingStatement.id() => "Expecting statement before {1}.".into(),
        DiagnosticKind::Unexpected.id() => "Unexpected {1}.".into(),
        DiagnosticKind::XmlClosingTagNameMustBeEquals.id() => "Closing tag name must be equals '{1}'.".into(),
        // DiagnosticKind::K.id() => ".".into(),
    };
}
