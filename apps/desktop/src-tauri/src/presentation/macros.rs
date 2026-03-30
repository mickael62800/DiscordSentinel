/// Generates a Tauri pass-through command that delegates directly to a service method.
///
/// # Patterns
///
/// ```ignore
/// // No parameters
/// tauri_passthrough!(get_dashboard_stats, DashboardService, get_stats -> ServerStats);
///
/// // One or more parameters
/// tauri_passthrough!(get_logs, LogsService, get_logs -> Vec<LogEntry>, guild_id: Option<String>);
///
/// // Multiple parameters
/// tauri_passthrough!(execute_ban, ModerationService, execute_ban -> (), guild_id: String, user_id: String, reason: String);
/// ```
macro_rules! tauri_passthrough {
    // Pattern: no extra parameters
    ($cmd:ident, $svc_ty:ty, $method:ident -> $ret:ty) => {
        #[tauri::command]
        pub async fn $cmd(
            service: tauri::State<'_, std::sync::Arc<$svc_ty>>,
        ) -> Result<$ret, String> {
            service.$method().await
        }
    };

    // Pattern: one or more extra parameters
    ($cmd:ident, $svc_ty:ty, $method:ident -> $ret:ty, $($param:ident : $pty:ty),+ $(,)?) => {
        #[tauri::command]
        pub async fn $cmd(
            service: tauri::State<'_, std::sync::Arc<$svc_ty>>,
            $($param : $pty),+
        ) -> Result<$ret, String> {
            service.$method($($param),+).await
        }
    };
}
