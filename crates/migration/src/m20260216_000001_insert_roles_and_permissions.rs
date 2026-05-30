use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

macro_rules! query_rid {
    ($name:literal) => {
        Query::select()
            .columns([Roles::Rid])
            .from(Roles::Table)
            .and_where(Expr::col(Roles::Name).eq($name))
            .take()
            .into()
    };
}
macro_rules! query_pid {
    ($name:literal) => {
        Query::select()
            .columns([Permissions::Pid])
            .from(Permissions::Table)
            .and_where(Expr::col(Permissions::Name).eq($name))
            .take()
            .into()
    };
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let insert_roles = Query::insert()
      .into_table(Roles::Table)
      .columns([Roles::Name, Roles::Description, Roles::Builtin])
      .values_panic([
        "Administrator".into(),
        "Unrestricted administrative access to all server resources and system settings".into(),
        true.into(),
      ])
      .values_panic([
        "User".into(),
        "Standard authenticated member with full playback and personal collection management"
          .into(),
        true.into(),
      ])
      .values_panic([
        "Guest".into(),
        "Temporary or restricted observer with limited access to specific libraries".into(),
        true.into(),
      ])
      .values_panic([
        "Anonymous".into(),
        "Unauthenticated public access with strictly read-only playback if enabled".into(),
        true.into(),
      ])
      .to_owned();

        let insert_permissions = Query::insert()
      .into_table(Permissions::Table)
      .columns([Permissions::Name, Permissions::Description, Permissions::Builtin])
      .values_panic(["System.Power".into(),"Authorize critical power operations: Restart, Shutdown, or Sleep the server host.".into(), true.into()])
      .values_panic(["System.Update".into(),"Check for, download, and initiate core system software or firmware updates.".into(), true.into()])
      .values_panic(["System.LogView".into(),"Access real-time streaming logs and historical error logs for debugging and auditing.".into(), true.into()])
      .values_panic(["System.Wipe".into(),"DANGER: Authorize factory reset or complete erasure of all database records and user data.".into(), true.into()])
      .values_panic(["System.Config".into(),"Modify network settings, port forwarding, and global server-side configurations.".into(), true.into()])
      .values_panic(["System.Backup".into(),"Create, download, or restore database and configuration backups.".into(), true.into()])
      .values_panic(["System.About".into(),"View the project README documentation and about page.".into(), true.into()])

      .values_panic(["Plugin.Install".into(),"Browse the marketplace and install new third-party extensions or drivers.".into(), true.into()])
      .values_panic(["Plugin.Manage".into(),"Enable, disable, or modify settings for currently installed plugins.".into(), true.into()])
      .values_panic(["Plugin.Uninstall".into(),"Completely remove plugin files and their associated cached data from the system.".into(), true.into()])

      .values_panic(["Library.Create".into(),"Define new media folders (e.g., \"Movies\", \"Anime\") and assign storage paths.".into(), true.into()])
      .values_panic(["Library.Scan".into(),"Manually trigger the media scanner to identify new files and update the library.".into(), true.into()])
      .values_panic(["Library.Delete".into(),"Remove an entire library category from the server (does not necessarily delete files).".into(), true.into()])
      .values_panic(["Library.View".into(),"Basic permission to browse and see content within assigned libraries.".into(), true.into()])
      .values_panic(["Library.Modify".into(),"Modify library data fields and manage associated library paths and settings.".into(), true.into()])
      .values_panic(["Library.PathView".into(),"Browse and select file system directory paths for library folder configuration.".into(), true.into()])
      .values_panic(["Scanner.Config".into(),"Configure metadata providers (TMDB, Fanart.tv) and scanning intervals.".into(), true.into()])
      .values_panic(["Scanner.DeepAnalyze".into(),"Trigger CPU-intensive tasks like generating video intro markers or chapter thumbnails.".into(), true.into()])

      .values_panic(["Metadata.Modify".into(),"Modify titles, descriptions, release dates, and cast information for any media item. (Extended information of resource)".into(), true.into()])
      .values_panic(["Metadata.Artwork".into(),"Upload or change posters, backdrops, and logo images for movies or shows. (Extended information of resource)".into(), true.into()])
      .values_panic(["Metadata.Lock".into(),"Prevent the automated scanner from overwriting manual changes to specific media items. (Extended information of resource)".into(), true.into()])
      .values_panic(["Metadata.View".into(),"View enriched media metadata including titles, descriptions, release dates, cast, and other media details (Extended information of resource).".into(), true.into()])
      .values_panic(["Tag.View".into(),"Browse, search, and view tags and tag groups used across the library for filtering and organization.".into(),true.into()])
      .values_panic(["Tag.Manage".into(),"Create, rename, or delete global tags and genres used for filtering and organization.".into(),true.into()])
      .values_panic(["Tag.Assign".into(),"Add or remove tags from specific media items without changing core metadata.".into(), true.into()])

      .values_panic(["User.View".into(),"List registered users and view their basic profile information.".into(), true.into()])
      .values_panic(["User.Create".into(),"Create new user accounts and send invitation links or temporary passwords.".into(), true.into()])
      .values_panic(["User.Delete".into(),"Terminate user accounts and purge their playback history and personal data.".into(), true.into()])
      .values_panic(["User.ManageRoles".into(),"Create, modify, and delete role definitions, and assign roles to users.".into(), true.into()])
      .values_panic(["User.ManagePerms".into(),"View, create, modify, and delete permission definitions, and manage permission-to-role bindings.".into(), true.into()])
      .values_panic(["User.ManageProfile".into(),"Modify user profile information including avatar, display name, and other personal settings for any user.".into(), true.into()])
      .values_panic(["User.Audit".into(),"View active sessions, IP addresses, and real-time playback activity of other users.".into(), true.into()])

      .values_panic(["Resource.View".into(), "Browse and search media resource entries in the library catalog.".into(), true.into()])
      .values_panic(["Resource.Create".into(), "Add new media resource records to the library database.".into(), true.into()])
      .values_panic(["Resource.Modify".into(), "Update existing resource records, file paths, or other resource-related settings.".into(), true.into()])
      .values_panic(["Resource.Delete".into(), "Remove resource records from the library database.".into(), true.into()])

      .values_panic(["Media.Download".into(),"Allow users to download original raw media files for local offline storage.".into(), true.into()])
      .values_panic(["Media.Upload".into(),"Upload media files or other content from client devices to the server.".into(), true.into()])
      .values_panic(["Media.Sync".into(),"Allow \"Sync for Mobile\" which involves background transcoding for offline playback.".into(), true.into()])
      .values_panic(["Media.DeleteFile".into(),"CRITICAL: Allow the physical deletion of media files from the hard drive via the UI.".into(), true.into()])
      .values_panic(["Stream.Transcode".into(),"Grant permission to use server GPU/CPU resources for real-time bitrate conversion.".into(), true.into()])
      .values_panic(["Stream.LiveTV".into(),"Allow access to Live TV tuners and DVR scheduling (if hardware is present).".into(), true.into()])
      .values_panic(["Social.Share".into(),"Create temporary public links or \"Party Mode\" invites for non-registered users.".into(), true.into()])
      .values_panic(["Remote.Control".into(),"Allow controlling other active client devices (Cast/DLNA) via the current session.".into(), true.into()])
      .to_owned();

        manager.execute(insert_roles).await?;
        manager.execute(insert_permissions).await?;

        // --- Administrator (All inclusive) ---
        let bind_administrator_permissions = Query::insert()
            .into_table(RolesRelatedPermissions::Table)
            .columns([
                RolesRelatedPermissions::Rid,
                RolesRelatedPermissions::Pid,
                RolesRelatedPermissions::Builtin,
            ])
            .select_from(
                Query::select()
                    .expr(
                        Query::select()
                            .column(Roles::Rid)
                            .from(Roles::Table)
                            .and_where(Expr::col(Roles::Name).eq("Administrator"))
                            .to_owned(),
                    )
                    .column(Permissions::Pid)
                    .expr(Expr::val(true))
                    .from(Permissions::Table)
                    .take(),
            )
            .unwrap()
            .take();

        // --- User (Consumption focus) ---
        let bind_user_permissions = Query::insert()
            .into_table(RolesRelatedPermissions::Table)
            .columns([
                RolesRelatedPermissions::Rid,
                RolesRelatedPermissions::Pid,
                RolesRelatedPermissions::Builtin,
            ])
            .values_panic([
                query_rid!("User"),
                query_pid!("Media.Download"),
                true.into(),
            ])
            .values_panic([query_rid!("User"), query_pid!("Media.Sync"), true.into()])
            .values_panic([
                query_rid!("User"),
                query_pid!("Stream.Transcode"),
                true.into(),
            ])
            .values_panic([query_rid!("User"), query_pid!("Metadata.View"), true.into()])
            .values_panic([query_rid!("User"), query_pid!("Library.View"), true.into()])
            .values_panic([query_rid!("User"), query_pid!("Tag.View"), true.into()])
            .values_panic([query_rid!("User"), query_pid!("Resource.View"), true.into()])
            .values_panic([query_rid!("User"), query_pid!("User.View"), true.into()])
            .to_owned();

        let bind_guest_permissions = Query::insert()
            .into_table(RolesRelatedPermissions::Table)
            .columns([
                RolesRelatedPermissions::Rid,
                RolesRelatedPermissions::Pid,
                RolesRelatedPermissions::Builtin,
            ])
            .values_panic([
                query_rid!("Guest"),
                query_pid!("Media.Download"),
                true.into(),
            ])
            .to_owned();

        manager.execute(bind_administrator_permissions).await?;
        manager.execute(bind_user_permissions).await?;
        manager.execute(bind_guest_permissions).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let delete_roles = Query::delete().from_table(Roles::Table).to_owned();
        let delete_permissions = Query::delete().from_table(Permissions::Table).to_owned();

        manager.execute(delete_roles).await?;
        manager.execute(delete_permissions).await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Roles {
    Table,
    Rid,
    Name,
    Description,
    Builtin,
}

#[derive(DeriveIden)]
enum Permissions {
    Table,
    Pid,
    Name,
    Description,
    Builtin,
}

#[derive(DeriveIden)]
enum RolesRelatedPermissions {
    Table,
    Rid,
    Pid,
    Builtin,
}
