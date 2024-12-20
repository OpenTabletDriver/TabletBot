use crate::{commands::ERROR_COLOUR, Data};
use poise::serenity_prelude::{
    self as serenity, ChannelId, CreateEmbed, CreateMessage, GuildChannel,
};

pub async fn thread_create(ctx: &serenity::Context, data: &Data, thread: &GuildChannel) {
    // TODO: make this an environment variable?
    if thread.parent_id != Some(ChannelId::from(1149679256981479464)) {
        return;
    }

    // Already responded to this thread.
    if data.forum_threads.read().unwrap().contains(&thread.id) {
        return;
    }

    // This is jank until serenity 0.13 or switching tablet bot to serenity@next.
    // This will prevent the bot from responding if it just joins the thread without it being a new thread.
    if chrono::Utc::now() - *thread.id.created_at() > chrono::Duration::seconds(5) {
        return;
    }

    data.forum_threads.write().unwrap().insert(thread.id);
    collect_and_action(ctx, thread).await;
}

macro_rules! check_device {
    ($device:expr, $vendor_id:expr, $pattern:pat $(if $guard:expr)?) => {
        (($device.vendor_id == $vendor_id) && matches!($device.product_id, $pattern $(if $guard)?))
    };
}

async fn collect_and_action(ctx: &serenity::Context, thread: &GuildChannel) {
    if let Some(message) = serenity::MessageCollector::new(ctx)
        .timeout(std::time::Duration::from_secs(10))
        .channel_id(thread.id)
        .await
    {
        println!("{}", message.id);

        if message.attachments.is_empty() {
            let desc = "Sending diagnostics is a mandatory step! Please follow the instructions \
                        below or this request will be deleted.\n\n- Start OpenTabletDriver (if it \
                        is not already running)\n- Go to `Help` -> `Export Diagnostics` in the \
                        top menu\n- Save the file, then upload here.";

            let embed = CreateEmbed::new()
                .title("Exporting diagnostics")
                .description(desc)
                .color(ERROR_COLOUR);

            let _ = thread
                .id
                .send_message(ctx, CreateMessage::new().embed(embed))
                .await;
            return;
        }

        for attachment in message.attachments {
            // 5MB, unlikely to be a diagnostic and massive if it is one.
            if attachment.size > 5000000 {
                println!("big attachment");
                continue;
            }

            let will_eval = match attachment.content_type {
                Some(ref s) if s.contains("application/json") => true,
                Some(ref s) if s.contains("text/plain") => true,
                // user likely stripped the extension
                None => true,
                // something else, likely an image
                _ => false,
            };

            if will_eval {
                let Ok(data) = attachment.download().await else {
                    println!("Failed to download attachment, message was likely deleted.");
                    return;
                };

                let Ok(diag) = serde_json::from_slice::<MinimifiedDiagSchema>(&data) else {
                    println!("Could not parse attachment as diagnostic.");
                    continue;
                };

                let mut maybe_dump = None;
                // Checks if it is a known problematic device that needs a string dump
                // Right now it just ends at the first mention of a problematic device, but not
                // many people ask for config support when they have multiple tablets plugged in?
                for device in diag.hid_devices {
                    if check_device!(device, 21827, 129) || check_device!(device, 9580, 90..120) {
                        maybe_dump = Some(device);
                        break;
                    }
                }

                if let Some(device) = maybe_dump {
                    let description = format!(
                        "Your device is known to have tricky identifiers to work with, and such a \
                         device string dump will help support this tablet faster. Please follow \
                         these instructions below.\n\n- Start OpenTabletDriver (if it is not \
                         already running)\n- Go to `Tablets` -> `Device string reader` in the top \
                         menu\n- Put `{}` in the top box\n- `{}` in the middle box\n- Press `Dump \
                         all`\n- Save the file, then upload here.",
                        device.vendor_id, device.product_id
                    );

                    let embed = CreateEmbed::new()
                        .title("String dump required")
                        .description(description)
                        .color(ERROR_COLOUR);

                    let _ = thread
                        .send_message(ctx, CreateMessage::new().embed(embed))
                        .await;
                    break;
                }
            }
        }
    }
}

#[derive(serde::Deserialize)]
struct MinimifiedDiagSchema {
    #[serde(rename = "HID Devices")]
    hid_devices: Vec<HidDevice>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HidDevice {
    #[serde(rename = "VendorID")]
    vendor_id: i16,
    #[serde(rename = "ProductID")]
    product_id: i16,
    // I will put these in when I have a "surefire" way of getting the intended device.
    // I will probably use them for checking if udev rules are installed (to get the right lengths)
    //input_report_length: i16,
    //output_report_length: i16,
}
