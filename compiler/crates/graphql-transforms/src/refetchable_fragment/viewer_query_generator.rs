/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use super::{
    build_fragment_spread, build_operation_variable_definitions, QueryGenerator, RefetchRoot,
    CONSTANTS,
};
use crate::root_variables::VariableMap;
use common::WithLocation;
use graphql_ir::{
    FragmentDefinition, LinkedField, OperationDefinition, Selection, ValidationError,
    ValidationMessage, ValidationResult,
};
use graphql_syntax::OperationKind;
use interner::StringKey;
use schema::{FieldID, Schema, Type};
use std::sync::Arc;

fn build_refetch_operation(
    schema: &Schema,
    fragment: &Arc<FragmentDefinition>,
    query_name: StringKey,
    variables_map: &VariableMap,
) -> ValidationResult<Option<RefetchRoot>> {
    if schema.get_type_name(fragment.type_condition) != CONSTANTS.viewer_type_name {
        return Ok(None);
    }
    let query_type = schema.query_type().unwrap();
    let viewer_field_id = get_viewer_field_id(schema, query_type, fragment)?;

    Ok(Some(RefetchRoot {
        identifier_field: None,
        path: vec![CONSTANTS.viewer_field_name],
        operation: Arc::new(OperationDefinition {
            kind: OperationKind::Query,
            name: WithLocation::new(fragment.name.location, query_name),
            type_: query_type,
            variable_definitions: build_operation_variable_definitions(
                variables_map,
                &fragment.variable_definitions,
            ),
            directives: vec![],
            selections: vec![Selection::LinkedField(Arc::new(LinkedField {
                alias: None,
                definition: WithLocation::new(fragment.name.location, viewer_field_id),
                arguments: vec![],
                directives: vec![],
                selections: vec![build_fragment_spread(fragment)],
            }))],
        }),
        fragment: Arc::clone(fragment),
    }))
}

fn get_viewer_field_id(
    schema: &Schema,
    query_type: Type,
    fragment: &FragmentDefinition,
) -> ValidationResult<FieldID> {
    let viewer_type = schema.get_type(CONSTANTS.viewer_type_name);
    let viewer_field_id = schema.named_field(query_type, CONSTANTS.viewer_field_name);
    if let Some(viewer_type) = viewer_type {
        if let Some(viewer_field_id) = viewer_field_id {
            let viewer_field = schema.field(viewer_field_id);
            if viewer_type.is_object()
                && viewer_type == viewer_field.type_.inner()
                && viewer_type == fragment.type_condition
                && viewer_field.arguments.is_empty()
            {
                return Ok(viewer_field_id);
            }
        }
    }
    Err(vec![ValidationError::new(
        ValidationMessage::InvalidViewerSchemaForRefetchableFragmentOnViewer {
            fragment_name: fragment.name.item,
        },
        vec![fragment.name.location],
    )])
}

pub const VIEWER_QUERY_GENERATOR: QueryGenerator = QueryGenerator {
    description: "the Viewer type",
    build_refetch_operation,
};