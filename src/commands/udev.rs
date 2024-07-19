use crate::{Context, Error};

use std::fmt::Write;

use poise::serenity_prelude::CreateAttachment;
use poise::CreateReply;

/// Generates udev rules for the given vendor and product Ids.
#[poise::command(
    rename = "generate-udev",
    aliases("udev"),
    slash_command,
    prefix_command
)]
pub async fn generate_udev(
    ctx: Context<'_>,
    #[description = "The Vendor Id in decimal."] vendor_id: u64,
    #[description = "The Product Id in decimal."] product_id: u64,
    libinput_override: Option<bool>,
) -> Result<(), Error> {
    let udev = gen_udev(vendor_id, product_id, libinput_override.unwrap_or(true));

    let attachment = CreateAttachment::bytes(udev, "70-opentabletdriver.rules");
    ctx.send(
        CreateReply::default()
            .content("place this file in `/etc/udev/rules.d/70-opentabletdriver.rules` then run the following:\n \
            ```\nsudo udevadm control --reload-rules && sudo udevadm trigger\n```")
            .attachment(attachment),
    )
    .await?;

    Ok(())
}

const REQUIRED_UDEV_STR: &str = r#"
KERNEL=="uinput", SUBSYSTEM=="misc", OPTIONS+="static_node=uinput", TAG+="uaccess", TAG+="udev-acl"
KERNEL=="js[0-9]*", SUBSYSTEM=="input", ATTRS{name}=="OpenTabletDriver Virtual Tablet", RUN+="/usr/bin/env rm %E{DEVNAME}"
"#;

fn gen_udev(id_vendor: u64, id_product: u64, libinput_override: bool) -> String {
    let mut udev_rules = format!(
        "KERNEL==\"hidraw*\", ATTRS{{idVendor}}==\"{id_vendor:X}\", ATTRS{{idProduct}}==\"{id_product:X}\", TAG+=\"uaccess\", TAG+=\"udev-acl\"\n\
        SUBSYSTEM==\"usb\", ATTRS{{idVendor}}==\"{id_vendor:X}\", ATTRS{{idProduct}}==\"{id_product:X}\", TAG+=\"uaccess\", TAG+=\"udev-acl\""
    );

    if libinput_override {
        write!(
            udev_rules,
            "\nSUBSYSTEM==\"input\", ATTRS{{idVendor}}==\"{id_vendor:X}\", ATTRS{{idProduct}}==\"{id_product:X}\""
        ).unwrap();
    }

    format!("{REQUIRED_UDEV_STR}\n# Generated by TabletBot\n{udev_rules}")
}
