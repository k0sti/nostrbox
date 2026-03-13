use crate::{Actor, GlobalRole, Scope, Visibility};

/// Check if an actor can access a resource with the given scope.
pub fn can_access(actor: &Actor, scope: &Scope) -> bool {
    match scope.visibility {
        Visibility::Public => true,
        Visibility::Internal => actor.global_role == GlobalRole::Owner,
        Visibility::Group => {
            let Some(ref group_id) = scope.group_id else {
                return false;
            };
            actor.groups.iter().any(|g| g == group_id)
        }
        // Reserved — deny for now
        Visibility::Circle | Visibility::Personal => false,
    }
}
