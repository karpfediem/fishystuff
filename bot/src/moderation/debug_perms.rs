use poise::serenity_prelude as serenity;
use serenity::{
    model::guild::{Member, Role},
    model::permissions::Permissions,
};
use std::collections::HashMap;

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn debug_kickability(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    target_id: serenity::UserId,
) -> Result<(), Error> {
    let http = &ctx.http;

    let guild = guild_id.to_partial_guild(http).await?;
    let roles: HashMap<serenity::RoleId, Role> = guild_id.roles(http).await?;

    let bot_id = ctx.cache.current_user().id;

    let bot_member: Member = guild_id.member(http, bot_id).await?;
    let target_member: Member = guild_id.member(http, target_id).await?;

    let bot_perms = effective_guild_perms(&bot_member, guild_id, &roles);
    let target_top = highest_role_pos(&target_member, &roles);
    let bot_top = highest_role_pos(&bot_member, &roles);

    let can_kick_perm = bot_perms.kick_members() || bot_perms.administrator();
    let hierarchy_ok = bot_top > target_top;
    let is_owner = target_id == guild.owner_id;
    let is_self = target_id == bot_id;

    tracing::warn!(
        "kickability: bot_perm_kick={} admin={} bot_top={} target_top={} hierarchy_ok={} is_owner={} is_self={}",
        can_kick_perm,
        bot_perms.administrator(),
        bot_top,
        target_top,
        hierarchy_ok,
        is_owner,
        is_self
    );

    if !can_kick_perm {
        tracing::warn!("Missing Kick Members (or Administrator) on the bot’s roles.");
    }
    if !hierarchy_ok {
        tracing::warn!(
            "Role hierarchy blocks action: raise the bot’s top role above the target’s."
        );
    }
    if is_owner {
        tracing::warn!("Cannot kick the guild owner.");
    }
    if is_self {
        tracing::warn!("Refusing to kick self/bot.");
    }

    Ok(())
}

fn effective_guild_perms(
    member: &Member,
    guild_id: serenity::GuildId,
    roles: &HashMap<serenity::RoleId, Role>,
) -> Permissions {
    // Start with @everyone (role id == guild id)
    let mut perms = roles
        .get(&serenity::RoleId::new(guild_id.get()))
        .map(|r| r.permissions)
        .unwrap_or_else(Permissions::empty);

    // OR all assigned roles
    for rid in &member.roles {
        if let Some(r) = roles.get(rid) {
            perms |= r.permissions;
        }
    }

    // Administrator implies all permissions (still subject to hierarchy rules!)
    if perms.administrator() {
        Permissions::all()
    } else {
        perms
    }
}

fn highest_role_pos(member: &Member, roles: &HashMap<serenity::RoleId, Role>) -> u16 {
    member
        .roles
        .iter()
        .filter_map(|rid| roles.get(rid))
        .map(|r| r.position)
        .max()
        .unwrap_or(0)
}
