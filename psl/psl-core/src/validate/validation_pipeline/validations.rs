mod autoincrement;
mod composite_types;
mod constraint_namespace;
mod database_name;
mod default_value;
mod enums;
mod fields;
mod indexes;
mod models;
mod names;
mod relation_fields;
mod relations;

use super::context::Context;
use diagnostics::DatamodelError;
use names::Names;
use parser_database::walkers::RefinedRelationWalker;

pub(super) fn validate(ctx: &mut Context<'_>) {
    let db = ctx.db;
    let connector = ctx.connector;

    let names = Names::new(db, connector);

    composite_types::detect_composite_cycles(ctx);
    for composite_type in db.walk_composite_types() {
        composite_types::composite_types_support(composite_type, ctx);

        if !ctx.diagnostics.has_errors() {
            composite_types::more_than_one_field(composite_type, ctx);

            for field in composite_type.fields() {
                composite_types::validate_default_value(field, ctx);
                fields::validate_native_type_arguments(field, ctx);
            }
        }
    }

    // Model validations
    ctx.connector
        .validate_scalar_field_unknown_default_functions(ctx.db, ctx.diagnostics);

    for model in db.walk_models() {
        models::has_a_strict_unique_criteria(model, ctx);
        models::has_a_unique_primary_key_name(model, &names, ctx);
        models::has_a_unique_custom_primary_key_name_per_model(model, &names, ctx);
        models::id_has_fields(model, ctx);
        models::id_client_name_does_not_clash_with_field(model, ctx);
        models::primary_key_connector_specific(model, ctx);
        models::primary_key_length_prefix_supported(model, ctx);
        models::primary_key_sort_order_supported(model, ctx);
        models::only_one_fulltext_attribute_allowed(model, ctx);
        models::connector_specific(model, ctx);
        autoincrement::validate_auto_increment(model, ctx);

        if let Some(pk) = model.primary_key() {
            for field_attribute in pk.scalar_field_attributes() {
                let span = pk.ast_attribute().span;
                let attribute = (pk.attribute_name(), span);
                fields::validate_length_used_with_correct_types(field_attribute, attribute, ctx);
            }

            fields::id_supports_clustering_setting(pk, ctx);
            fields::clustering_can_be_defined_only_once(pk, ctx);
        }

        for field in model.scalar_fields() {
            fields::validate_scalar_field_connector_specific(field, ctx);
            fields::validate_client_name(field.into(), &names, ctx);
            fields::has_a_unique_default_constraint_name(field, &names, ctx);
            fields::validate_native_type_arguments(field, ctx);
            fields::validate_default_value(field, ctx);
            fields::validate_unsupported_field_type(field, ctx)
        }

        for field in model.relation_fields() {
            // We don't want to spam, so with ambiguous relations we should exit
            // immediately if any errors.
            if let Err(error) = relation_fields::ambiguity(field, &names) {
                ctx.push_error(error);
                return;
            }

            fields::validate_client_name(field.into(), &names, ctx);

            relation_fields::ignored_related_model(field, ctx);
            relation_fields::referential_actions(field, ctx);
            relation_fields::map(field, ctx);
        }

        for index in model.indexes() {
            indexes::has_fields(index, ctx);
            indexes::has_a_unique_constraint_name(index, &names, ctx);
            indexes::unique_client_name_does_not_clash_with_field(index, ctx);
            indexes::unique_index_has_a_unique_custom_name_per_model(index, &names, ctx);
            indexes::field_length_prefix_supported(index, ctx);
            indexes::index_algorithm_is_supported(index, ctx);
            indexes::hash_index_must_not_use_sort_param(index, ctx);
            indexes::fulltext_index_preview_feature_enabled(index, ctx);
            indexes::fulltext_index_supported(index, ctx);
            indexes::fulltext_columns_should_not_define_length(index, ctx);
            indexes::fulltext_column_sort_is_supported(index, ctx);
            indexes::fulltext_text_columns_should_be_bundled_together(index, ctx);
            indexes::has_valid_mapped_name(index, ctx);
            indexes::supports_clustering_setting(index, ctx);
            indexes::clustering_can_be_defined_only_once(index, ctx);
            indexes::opclasses_are_not_allowed_with_other_than_normal_indices(index, ctx);

            for field_attribute in index.scalar_field_attributes() {
                let span = index.ast_attribute().span;
                let attribute = (index.attribute_name(), span);

                fields::validate_length_used_with_correct_types(field_attribute, attribute, ctx);
            }
        }
    }

    if !connector.supports_enums() {
        for r#enum in db.ast().iter_tops().filter_map(|(_, top)| top.as_enum()) {
            ctx.push_error(DatamodelError::new_validation_error(
                &format!(
                    "You defined the enum `{}`. But the current connector does not support enums.",
                    &r#enum.name.name
                ),
                r#enum.span,
            ));
        }
    } else {
        for r#enum in db.walk_enums() {
            ctx.connector.validate_enum(r#enum, ctx.diagnostics);
        }
    }

    if !ctx.preview_features.contains(crate::PreviewFeature::MultiSchema) {
        for model in ctx.db.walk_models() {
            if let Some((_, span)) = model.schema() {
                ctx.push_error(DatamodelError::new_static(
                    "@@schema is only available with the `multiSchema` preview feature.",
                    span,
                ))
            }
        }

        for enm in ctx.db.walk_enums() {
            if let Some((_, span)) = enm.schema() {
                ctx.push_error(DatamodelError::new_static(
                    "@@schema is only available with the `multiSchema` preview feature.",
                    span,
                ))
            }
        }

        if let Some(ds) = ctx.datasource {
            if let Some(span) = ds.schemas_span {
                ctx.push_error(DatamodelError::new_static(
                    "The `schemas` property is only availably with the `multiSchema` preview feature.",
                    span,
                ))
            }
        }
    } else {
        for model in ctx.db.walk_models() {
            models::schema_attribute(model, ctx);
        }

        for r#enum in ctx.db.walk_enums() {
            enums::schema_attribute(r#enum, ctx);
        }

        if !ctx
            .connector
            .has_capability(crate::datamodel_connector::ConnectorCapability::MultiSchema)
        {
            if let Some(ds) = ctx.datasource {
                if let Some(span) = ds.schemas_span {
                    ctx.push_error(DatamodelError::new_static(
                        "The `schemas` property is not supported on the current connector.",
                        span,
                    ))
                }
            }
        }
    }

    for relation in db.walk_relations() {
        match relation.refine() {
            // 1:1, 1:n
            RefinedRelationWalker::Inline(relation) => {
                if let Some(relation) = relation.as_complete() {
                    relations::cycles(relation, ctx);
                    relations::multiple_cascading_paths(relation, ctx);
                }

                relations::references_unique_fields(relation, ctx);
                relations::same_length_in_referencing_and_referenced(relation, ctx);
                relations::field_arity(relation, ctx);
                relations::referencing_scalar_field_types(relation, ctx);
                relations::has_a_unique_constraint_name(&names, relation, ctx);

                if relation.is_one_to_one() {
                    relations::one_to_one::both_sides_are_defined(relation, ctx);
                    relations::one_to_one::fields_and_references_are_defined(relation, ctx);
                    relations::one_to_one::fields_and_references_defined_on_one_side_only(relation, ctx);
                    relations::one_to_one::referential_actions(relation, ctx);
                    relations::one_to_one::fields_must_be_a_unique_constraint(relation, ctx);
                    relations::one_to_one::fields_references_mixups(relation, ctx);
                    relations::one_to_one::back_relation_arity_is_optional(relation, ctx);
                    relations::one_to_one::fields_and_references_on_wrong_side(relation, ctx);
                } else {
                    relations::one_to_many::both_sides_are_defined(relation, ctx);
                    relations::one_to_many::fields_and_references_are_defined(relation, ctx);
                    relations::one_to_many::referential_actions(relation, ctx);
                }
            }
            RefinedRelationWalker::ImplicitManyToMany(relation) => {
                use relations::many_to_many::implicit;

                implicit::supports_implicit_relations(relation, ctx);
                implicit::validate_singular_id(relation, ctx);
                implicit::validate_no_referential_actions(relation, ctx);
                implicit::cannot_define_references_argument(relation, ctx);
            }
            RefinedRelationWalker::TwoWayEmbeddedManyToMany(relation) => {
                use relations::many_to_many::embedded;

                embedded::supports_embedded_relations(relation, ctx);
                embedded::defines_references_on_both_sides(relation, ctx);
                embedded::defines_fields_on_both_sides(relation, ctx);
                embedded::references_id_from_both_sides(relation, ctx);
                embedded::referencing_with_an_array_field_of_correct_type(relation, ctx);
                embedded::validate_no_referential_actions(relation, ctx);
            }
        }
    }
}
