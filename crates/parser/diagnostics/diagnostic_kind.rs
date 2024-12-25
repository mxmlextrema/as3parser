#[repr(i32)]
#[derive(Eq, PartialEq, Clone, Copy)]
pub enum DiagnosticKind {
    InvalidEscapeValue = 1024,
    UnexpectedEnd = 1025,
    UnallowedNumericSuffix = 1026,
    StringLiteralMustBeTerminatedBeforeLineBreak = 1027,
    Expecting = 1028,
    ExpectingIdentifier = 1029,
    ExpectingExpression = 1030,
    ExpectingXmlName = 1031,
    ExpectingXmlAttributeValue = 1032,
    IllegalNullishCoalescingLeftOperand = 1033,
    WrongParameterPosition = 1034,
    DuplicateRestParameter = 1035,
    NotAllowedHere = 1036,
    MalformedRestParameter = 1037,
    IllegalForInInitializer = 1038,
    MultipleForInBindings = 1039,
    UndefinedLabel = 1040,
    IllegalContinue = 1041,
    IllegalBreak = 1042,
    ExpressionMustNotFollowLineBreak = 1043,
    TokenMustNotFollowLineBreak = 1044,
    ExpectingStringLiteral = 1045,
    DuplicateAttribute = 1046,
    DuplicateAccessModifier = 1047,
    ExpectingDirectiveKeyword = 1048,
    UnallowedAttribute = 1049,
    UseDirectiveMustContainPublic = 1050,
    MalformedEnumMember = 1051,
    FunctionMayNotBeGenerator = 1052,
    FunctionMayNotBeAsynchronous = 1053,
    FunctionMustNotContainBody = 1054,
    FunctionMustContainBody = 1055,
    FunctionMustNotContainAnnotations = 1056,
    NestedClassesNotAllowed = 1057,
    UnexpectedDirective = 1058,
    FailedParsingAsdocTag = 1059,
    UnrecognizedAsdocTag = 1060,
    UnrecognizedProxy = 1061,
    EnumMembersMustBeConst = 1062,
    ConstructorMustNotSpecifyResultType = 1063,
    UnrecognizedMetadataSyntax = 1064,
    FailedToIncludeFile = 1065,
    ParentSourceIsNotAFile = 1066,
    CircularIncludeDirective = 1067,
    MalformedDestructuring = 1068,
    XmlPrefixNotDefined = 1069,
    RedefiningXmlAttribute = 1070,
    InvalidXmlPi = 1071,
    XmlPiUnknownAttribute = 1072,
    XmlPiVersion = 1073,
    XmlPiEncoding = 1074,
    XmlMustConsistOfExactly1Element = 1075,
    XmlNameAtMostOneColon = 1076,
    UnexpectedCharacter = 1077,
    InputEndedBeforeReachingClosingQuoteForString = 1078,
    InputEndedBeforeReachingClosingSeqForCData = 1079,
    InputEndedBeforeReachingClosingSeqForPi = 1080,
    InputEndedBeforeReachingClosingSeqForXmlComment = 1081,
    InputEndedBeforeReachingClosingSeqForMultiLineComment = 1082,
    InputEndedBeforeReachingClosingSlashForRegExp = 1083,
    InputEndedBeforeReachingClosingQuoteForAttributeValue = 1084,
    ExpectingEitherSemicolonOrNewLineHere = 1085,
    CssInvalidHexEscape = 1086,
    ExpectingDirective = 1087,
    ExpectingStatement = 1088,
    Unexpected = 1089,
    XmlClosingTagNameMustBeEquals = 1090,
    UnexpectedIncludeExtension = 1091,
    UnallowedExpression = 1092,
}

impl DiagnosticKind {
    pub fn id(&self) -> i32 {
        *self as i32
    }
}