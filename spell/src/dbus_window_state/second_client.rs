use zbus::proxy;

#[proxy(
    default_path = "/org/VimYoung/VarHandler",
    default_service = "org.VimYoung.Spell",
    interface = "org.VimYoung.Spell1"
)]
pub trait SecondClient {
    #[zbus(signal)]
    fn layer_var_value_changed(
        &self,
        layer_name: &str,
        var_name: &str,
        value: &str,
    ) -> zbus::Result<()>;
}
