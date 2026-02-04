//! Folder management routes.

use crate::middlewares::UserId;
use crate::models::{CreateFolderRequest, MoveFolderRequest, UpdateFolderRequest};
use crate::repository::{
    create_folder as repo_create, delete_folder as repo_delete,
    get_folder_with_contents, get_root_folders, get_subfolders, get_user_folder_by_id,
    move_folder as repo_move, update_folder_name,
};
use crate::utils::{e400, e404, e500};
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

/// Create a new folder.
#[tracing::instrument(name = "Create folder", skip(pool, body))]
pub async fn create_folder(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<CreateFolderRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let request = body.into_inner();

    // Validate name
    if request.name.is_empty() {
        return Err(e400("Folder name cannot be empty"));
    }
    if request.name.contains('/') {
        return Err(e400("Folder name cannot contain '/'"));
    }

    let folder = repo_create(&request.name, request.parent_id, &user_id, &pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                e400("A folder with this name already exists at this location")
            } else {
                e500(e)
            }
        })?;

    Ok(HttpResponse::Created().json(folder))
}

/// List root folders.
#[tracing::instrument(name = "List root folders", skip(pool))]
pub async fn list_root_folders(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let folders = get_root_folders(&user_id, &pool).await.map_err(e500)?;
    Ok(HttpResponse::Ok().json(folders))
}

/// Get folder by ID with contents.
#[tracing::instrument(name = "Get folder", skip(pool))]
pub async fn get_folder(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let folder_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    let folder = get_folder_with_contents(folder_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Folder not found"))?;

    Ok(HttpResponse::Ok().json(folder))
}

/// Get subfolders of a folder.
#[tracing::instrument(name = "Get subfolders", skip(pool))]
pub async fn get_folder_subfolders(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let folder_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    // Verify folder exists and belongs to user
    let _ = get_user_folder_by_id(folder_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Folder not found"))?;

    let subfolders = get_subfolders(folder_id, &user_id, &pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok().json(subfolders))
}

/// Update a folder (rename).
#[tracing::instrument(name = "Update folder", skip(pool, body))]
pub async fn update_folder(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<UpdateFolderRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let folder_id = parameters.into_inner();
    let user_id = user_id.into_inner();
    let request = body.into_inner();

    // Validate name if provided
    if let Some(ref name) = request.name {
        if name.is_empty() {
            return Err(e400("Folder name cannot be empty"));
        }
        if name.contains('/') {
            return Err(e400("Folder name cannot contain '/'"));
        }
    }

    // Check if folder exists
    let _ = get_user_folder_by_id(folder_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Folder not found"))?;

    let folder = if let Some(name) = request.name {
        update_folder_name(folder_id, &name, &user_id, &pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("duplicate key") {
                    e400("A folder with this name already exists at this location")
                } else {
                    e500(e)
                }
            })?
    } else {
        // No changes requested, return existing folder
        get_user_folder_by_id(folder_id, &user_id, &pool)
            .await
            .map_err(e500)?
            .unwrap()
    };

    Ok(HttpResponse::Ok().json(folder))
}

/// Move a folder to a new parent.
#[tracing::instrument(name = "Move folder", skip(pool, body))]
pub async fn move_folder(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<MoveFolderRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let folder_id = parameters.into_inner();
    let user_id = user_id.into_inner();
    let request = body.into_inner();

    let folder = repo_move(folder_id, request.parent_id, &user_id, &pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Cannot move folder into itself") ||
               msg.contains("Cannot move folder into its own descendant") {
                e400(msg)
            } else if msg.contains("not found") {
                e404(msg)
            } else if msg.contains("duplicate key") {
                e400("A folder with this name already exists at the target location")
            } else {
                e500(e)
            }
        })?;

    Ok(HttpResponse::Ok().json(folder))
}

/// Delete a folder.
#[tracing::instrument(name = "Delete folder", skip(pool))]
pub async fn delete_folder(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let folder_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    repo_delete(folder_id, &user_id, &pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                e404("Folder not found")
            } else {
                e500(e)
            }
        })?;

    Ok(HttpResponse::NoContent().finish())
}
