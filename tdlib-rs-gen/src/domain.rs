// Copyright 2026 - developers of the `tdlib-rs` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! 业务领域分类器（Domain Classifier）
//!
//! 将 TDLib 的数千个 AST 定义根据高内聚的业务领域划分为约 15 个模块，
//! 避免单文件数万行以及 3,200 个碎文件的两个极端，大幅提升编译速度与 IDE 索引流畅度。

/// 所有的核心业务领域列表（用于生成 mod.rs 声明与文件创建）
#[allow(dead_code)]
pub const ALL_DOMAINS: &[&str] = &[
    "bot",
    "call",
    "chat",
    "file",
    "forum",
    "message",
    "notification",
    "passport",
    "payment",
    "poll",
    "premium",
    "sticker",
    "story",
    "user",
    "misc",
];

/// 根据名称（结构体名、枚举名或函数名）判断其所属的业务领域
pub fn classify(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();

    // 1. 贴纸、表情与动效
    if lower.contains("sticker")
        || lower.contains("emoji")
        || lower.contains("dice")
        || lower.contains("animated")
    {
        return "sticker";
    }

    // 2. 身份与护照认证
    if lower.contains("passport")
        || lower.contains("identity_document")
        || lower.contains("personal_details")
        || lower.contains("bank_card")
    {
        return "passport";
    }

    // 3. 支付、订单与 Telegram Stars / Revenue
    if lower.contains("payment")
        || lower.contains("invoice")
        || lower.contains("stars")
        || lower.contains("star_")
        || lower.contains("revenue")
        || lower.contains("checkout")
        || lower.contains("receipt")
        || lower.contains("order_info")
        || lower.contains("shipping")
    {
        return "payment";
    }

    // 4. 故事（Story / Stories）
    if lower.contains("story") || lower.contains("stories") {
        return "story";
    }

    // 5. 会员特权、助推、赠品、商业功能与联盟营销
    if lower.contains("premium")
        || lower.contains("boost")
        || lower.contains("giveaway")
        || lower.contains("gift_code")
        || lower.contains("affiliate")
        || lower.contains("business")
    {
        return "premium";
    }

    // 6. 论坛与话题（Forum & Topics）
    if lower.contains("forum") || lower.contains("topic") {
        return "forum";
    }

    // 7. 投票与问卷（Poll）
    if lower.contains("poll") {
        return "poll";
    }

    // 8. 通话与音视频聊天（Call & Video Chat）
    if lower.contains("call") || lower.contains("video_chat") || lower.contains("group_call") {
        return "call";
    }

    // 9. 机器人与小程序（Bot, Web App & Inline Query）
    if lower.contains("bot")
        || lower.contains("inline_query")
        || lower.contains("callback_query")
        || lower.contains("web_app")
        || lower.contains("menu_button")
    {
        return "bot";
    }

    // 10. 通知设置与推送（Notification）
    if lower.contains("notification") || lower.contains("push_receiver") {
        return "notification";
    }

    // 11. 媒体与文件传输（File, Photo, Video, Audio, Document 等）
    if lower.contains("file")
        || lower.contains("photo")
        || lower.contains("video")
        || lower.contains("audio")
        || lower.contains("document")
        || lower.contains("voice_note")
        || lower.contains("animation")
        || lower.contains("thumbnail")
    {
        return "file";
    }

    // 12. 消息与富文本排版（Message, Text, Reaction 等）
    if lower.contains("message")
        || lower.contains("text")
        || lower.contains("draft")
        || lower.contains("caption")
        || lower.contains("reaction")
        || lower.contains("reply_markup")
    {
        return "message";
    }

    // 13. 对话群组与频道（Chat, Supergroup, BasicGroup 等）
    if lower.contains("chat")
        || lower.contains("supergroup")
        || lower.contains("basic_group")
        || lower.contains("secret_chat")
        || lower.contains("channel")
    {
        return "chat";
    }

    // 14. 用户个人资料与关系（User, Profile, Contact 等）
    if lower.contains("user")
        || lower.contains("profile")
        || lower.contains("contact")
        || lower == "get_me"
        || lower == "me"
    {
        return "user";
    }

    // 15. 其余基础协议与杂项（Misc）
    "misc"
}

/// 返回对应领域的 Rustdoc 简介
pub fn domain_description(domain: &str) -> &'static str {
    match domain {
        "bot" => "Types, enums, and functions for Telegram Bots, Web Apps, and inline queries.",
        "call" => "Types, enums, and functions for 1-on-1 calls, group calls, and live video chats.",
        "chat" => "Types, enums, and functions for managing private chats, basic groups, supergroups, and channels.",
        "file" => "Types, enums, and functions for local and remote files, photos, audio, videos, and documents.",
        "forum" => "Types, enums, and functions for supergroup forum topics and topic management.",
        "message" => "Types, enums, and functions for message content, rich formatting, reactions, and drafts.",
        "notification" => "Types, enums, and functions for push notifications and notification settings.",
        "passport" => "Types, enums, and functions for Telegram Passport and identity verification documents.",
        "payment" => "Types, enums, and functions for invoices, payments, Telegram Stars, and monetized features.",
        "poll" => "Types, enums, and functions for polls, quiz questions, and voter answers.",
        "premium" => "Types, enums, and functions for Telegram Premium, chat boosts, giveaways, and business features.",
        "sticker" => "Types, enums, and functions for stickers, custom emoji sets, and animated dice.",
        "story" => "Types, enums, and functions for Telegram Stories and story interactions.",
        "user" => "Types, enums, and functions for user accounts, privacy settings, and contacts.",
        _ => "Miscellaneous types, enums, network settings, and core TDLib options.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_classification() {
        assert_eq!(classify("Chat"), "chat");
        assert_eq!(classify("SendMessage"), "message");
        assert_eq!(classify("FormattedText"), "message");
        assert_eq!(classify("User"), "user");
        assert_eq!(classify("get_me"), "user");
        assert_eq!(classify("InputFile"), "file");
        assert_eq!(classify("GroupCall"), "call");
        assert_eq!(classify("StickerSet"), "sticker");
        assert_eq!(classify("ForumTopic"), "forum");
        assert_eq!(classify("CreateInvoiceLink"), "payment");
        assert_eq!(classify("SetTdlibParameters"), "misc");
    }
}
