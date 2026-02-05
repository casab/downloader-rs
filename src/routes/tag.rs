//! Tag management routes.

use crate::middlewares::UserId;
use crate::models::{AddTagsRequest, CreateTagRequest, Tag, UpdateTagRequest};
use crate::repository::{
    add_tags_to_download, create_tag as repo_create, delete_tag as repo_delete,
    get_download_tags, get_user_download_by_id, get_user_tag_by_id, get_user_tags,
    remove_tag_from_download, update_tag as repo_update,
};
use crate::utils::{e400, e404, e500};
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

/// Create a new tag.
#[tracing::instrument(name = "Create tag", skip(pool, body))]
pub async fn create_tag(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<CreateTagRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let request = body.into_inner();

    // Validate name
    Tag::validate_name(&request.name).map_err(|e| e400(e.to_string()))?;

    // Validate color if provided
    if let Some(ref color) = request.color {
        Tag::validate_color(color).map_err(|e| e400(e.to_string()))?;
    }

    let tag = repo_create(&request.name, request.color.as_deref(), &user_id, &pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                e400("A tag with this name already exists")
            } else {
                e500(e)
            }
        })?;

    Ok(HttpResponse::Created().json(tag))
}

/// List all tags for the user.
#[tracing::instrument(name = "List tags", skip(pool))]
pub async fn list_tags(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let tags = get_user_tags(&user_id, &pool).await.map_err(e500)?;
    Ok(HttpResponse::Ok().json(tags))
}

/// Get a tag by ID.
#[tracing::instrument(name = "Get tag", skip(pool))]
pub async fn get_tag(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let tag_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    let tag = get_user_tag_by_id(tag_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Tag not found"))?;

    Ok(HttpResponse::Ok().json(tag))
}

/// Update a tag.
#[tracing::instrument(name = "Update tag", skip(pool, body))]
pub async fn update_tag(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<UpdateTagRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let tag_id = parameters.into_inner();
    let user_id = user_id.into_inner();
    let request = body.into_inner();

    // Validate name if provided
    if let Some(ref name) = request.name {
        Tag::validate_name(name).map_err(|e| e400(e.to_string()))?;
    }

    // Validate color if provided
    if let Some(ref color) = request.color {
        Tag::validate_color(color).map_err(|e| e400(e.to_string()))?;
    }

    // Check if tag exists
    let _ = get_user_tag_by_id(tag_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Tag not found"))?;

    let tag = repo_update(
        tag_id,
        request.name.as_deref(),
        request.color.as_deref(),
        &user_id,
        &pool,
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate key") {
            e400("A tag with this name already exists")
        } else {
            e500(e)
        }
    })?;

    Ok(HttpResponse::Ok().json(tag))
}

/// Delete a tag.
#[tracing::instrument(name = "Delete tag", skip(pool))]
pub async fn delete_tag(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let tag_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    repo_delete(tag_id, &user_id, &pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                e404("Tag not found")
            } else {
                e500(e)
            }
        })?;

    Ok(HttpResponse::NoContent().finish())
}

/// Add tags to a download.
#[tracing::instrument(name = "Add tags to download", skip(pool, body))]
pub async fn add_download_tags(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<AddTagsRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let download_id = parameters.into_inner();
    let user_id = user_id.into_inner();
    let request = body.into_inner();

    if request.tag_ids.is_empty() {
        return Err(e400("No tags specified"));
    }

    let added = add_tags_to_download(download_id, &request.tag_ids, &user_id, &pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") {
                e404(msg)
            } else {
                e500(e)
            }
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "added": added })))
}

/// Remove a tag from a download.
#[tracing::instrument(name = "Remove tag from download", skip(pool))]
pub async fn remove_download_tag(
    parameters: web::Path<(uuid::Uuid, uuid::Uuid)>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let (download_id, tag_id) = parameters.into_inner();
    let user_id = user_id.into_inner();

    remove_tag_from_download(download_id, tag_id, &user_id, &pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") {
                e404(msg)
            } else {
                e500(e)
            }
        })?;

    Ok(HttpResponse::NoContent().finish())
}

/// Get tags for a download.
#[tracing::instrument(name = "Get download tags", skip(pool))]
pub async fn get_tags_for_download(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let download_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    // Verify download belongs to user
    let _ = get_user_download_by_id(download_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Download not found"))?;

    let tags = get_download_tags(download_id, &pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok().json(tags))
}
