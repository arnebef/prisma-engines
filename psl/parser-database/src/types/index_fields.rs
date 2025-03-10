use crate::{ast, types::SortOrder, DatamodelError};

pub(crate) enum OperatorClass<'a> {
    Constant(crate::OperatorClass),
    Raw(&'a str),
}

impl<'a> From<crate::OperatorClass> for OperatorClass<'a> {
    fn from(inner: crate::OperatorClass) -> Self {
        Self::Constant(inner)
    }
}

#[derive(Default)]
pub(crate) struct IndexFieldAttributes<'a> {
    pub(crate) field_name: &'a str,
    pub(crate) sort_order: Option<SortOrder>,
    pub(crate) length: Option<u32>,
    pub(crate) operator_class: Option<OperatorClass<'a>>,
}

struct FieldArguments<'a> {
    sort_order: Option<SortOrder>,
    length: Option<u32>,
    operator_class: Option<OperatorClass<'a>>,
}

pub(crate) fn coerce_field_array_with_args<'a>(
    expr: &'a ast::Expression,
    diagnostics: &mut diagnostics::Diagnostics,
) -> Option<Vec<IndexFieldAttributes<'a>>> {
    let f = |expr: &'a ast::Expression, diagnostics: &mut diagnostics::Diagnostics| -> Option<_> {
        match expr {
            ast::Expression::ConstantValue(field_name, _) => Some(IndexFieldAttributes {
                field_name,
                ..Default::default()
            }),
            ast::Expression::Function(field_name, args, _) => {
                let args = match field_args(&args.arguments) {
                    Ok(args) => args,
                    Err(err) => {
                        diagnostics.push_error(err);
                        return None;
                    }
                };

                let attrs = IndexFieldAttributes {
                    field_name,
                    sort_order: args.sort_order,
                    length: args.length,
                    operator_class: args.operator_class,
                };

                Some(attrs)
            }
            _ => {
                diagnostics.push_error(DatamodelError::new_type_mismatch_error(
                    "constant literal",
                    expr.describe_value_type(),
                    &expr.to_string(),
                    expr.span(),
                ));
                None
            }
        }
    };

    crate::coerce_array(expr, &f, diagnostics)
}

fn field_args(args: &[ast::Argument]) -> Result<FieldArguments<'_>, DatamodelError> {
    let sort_order = args
        .iter()
        .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("sort"))
        .map(|arg| match arg.value.as_constant_value() {
            Some(("Asc", _)) => Ok(Some(SortOrder::Asc)),
            Some(("Desc", _)) => Ok(Some(SortOrder::Desc)),
            None => Ok(None),
            _ => Err(DatamodelError::new_parser_error("Asc, Desc".to_owned(), arg.span)),
        })
        .transpose()?
        .flatten();

    let length = args
        .iter()
        .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("length"))
        .map(|arg| match &arg.value {
            ast::Expression::NumericValue(s, _) => s
                .parse::<u32>()
                .map_err(|_| DatamodelError::new_parser_error("valid integer".to_owned(), arg.span)),
            _ => Err(DatamodelError::new_parser_error("valid integer".to_owned(), arg.span)),
        })
        .transpose()?;

    let operator_class = args
        .iter()
        .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("ops"))
        .map(|arg| match &arg.value {
            ast::Expression::ConstantValue(s, span) => match s.as_str() {
                // gist
                "InetOps" => Ok(OperatorClass::from(crate::OperatorClass::InetOps)),

                // gin
                "JsonbOps" => Ok(OperatorClass::from(crate::OperatorClass::JsonbOps)),
                "JsonbPathOps" => Ok(OperatorClass::from(crate::OperatorClass::JsonbPathOps)),
                "ArrayOps" => Ok(OperatorClass::from(crate::OperatorClass::ArrayOps)),

                // sp-gist
                "TextOps" => Ok(OperatorClass::from(crate::OperatorClass::TextOps)),

                // brin
                "BitMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::BitMinMaxOps)),
                "VarBitMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::VarBitMinMaxOps)),
                "BpcharBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::BpcharBloomOps)),
                "BpcharMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::BpcharMinMaxOps)),
                "ByteaBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::ByteaBloomOps)),
                "ByteaMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::ByteaMinMaxOps)),
                "DateBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::DateBloomOps)),
                "DateMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::DateMinMaxOps)),
                "DateMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::DateMinMaxMultiOps)),
                "Float4BloomOps" => Ok(OperatorClass::from(crate::OperatorClass::Float4BloomOps)),
                "Float4MinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::Float4MinMaxOps)),
                "Float4MinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::Float4MinMaxMultiOps)),
                "Float8BloomOps" => Ok(OperatorClass::from(crate::OperatorClass::Float8BloomOps)),
                "Float8MinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::Float8MinMaxOps)),
                "Float8MinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::Float8MinMaxMultiOps)),
                "InetInclusionOps" => Ok(OperatorClass::from(crate::OperatorClass::InetInclusionOps)),
                "InetBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::InetBloomOps)),
                "InetMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::InetMinMaxOps)),
                "InetMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::InetMinMaxMultiOps)),
                "Int2BloomOps" => Ok(OperatorClass::from(crate::OperatorClass::Int2BloomOps)),
                "Int2MinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::Int2MinMaxOps)),
                "Int2MinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::Int2MinMaxMultiOps)),
                "Int4BloomOps" => Ok(OperatorClass::from(crate::OperatorClass::Int4BloomOps)),
                "Int4MinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::Int4MinMaxOps)),
                "Int4MinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::Int4MinMaxMultiOps)),
                "Int8BloomOps" => Ok(OperatorClass::from(crate::OperatorClass::Int8BloomOps)),
                "Int8MinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::Int8MinMaxOps)),
                "Int8MinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::Int8MinMaxMultiOps)),
                "NumericBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::NumericBloomOps)),
                "NumericMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::NumericMinMaxOps)),
                "NumericMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::NumericMinMaxMultiOps)),
                "OidBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::OidBloomOps)),
                "OidMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::OidMinMaxOps)),
                "OidMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::OidMinMaxMultiOps)),
                "TextBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::TextBloomOps)),
                "TextMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::TextMinMaxOps)),
                "TimestampBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampBloomOps)),
                "TimestampMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampMinMaxOps)),
                "TimestampMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampMinMaxMultiOps)),
                "TimestampTzBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampTzBloomOps)),
                "TimestampTzMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampTzMinMaxOps)),
                "TimestampTzMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::TimestampTzMinMaxMultiOps)),
                "TimeBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeBloomOps)),
                "TimeMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeMinMaxOps)),
                "TimeMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeMinMaxMultiOps)),
                "TimeTzBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeTzBloomOps)),
                "TimeTzMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeTzMinMaxOps)),
                "TimeTzMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::TimeTzMinMaxMultiOps)),
                "UuidBloomOps" => Ok(OperatorClass::from(crate::OperatorClass::UuidBloomOps)),
                "UuidMinMaxOps" => Ok(OperatorClass::from(crate::OperatorClass::UuidMinMaxOps)),
                "UuidMinMaxMultiOps" => Ok(OperatorClass::from(crate::OperatorClass::UuidMinMaxMultiOps)),

                s => Err(DatamodelError::new_parser_error(
                    format!("Invalid operator class: {s}"),
                    *span,
                )),
            },
            ast::Expression::Function(fun, args, span) => match fun.as_str() {
                "raw" => match args.arguments.as_slice() {
                    [arg] => match &arg.value {
                        ast::Expression::StringValue(s, _) => Ok(OperatorClass::Raw(s.as_str())),
                        _ => Err(DatamodelError::new_parser_error(
                            "Invalid parameter type: expected string".into(),
                            *span,
                        )),
                    },
                    args => Err(DatamodelError::new_parser_error(
                        format!("Wrong number of arguments. Expected: 1, got: {}", args.len()),
                        *span,
                    )),
                },
                _ => panic!(),
            },
            _ => Err(DatamodelError::new_parser_error("operator class".to_owned(), arg.span)),
        })
        .transpose()?;

    Ok(FieldArguments {
        sort_order,
        length,
        operator_class,
    })
}
