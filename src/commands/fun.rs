use crate::constants::emojis;
use crate::models::Message;
use crate::rest::RestClient;

const RESPONSES: &[&str] = &[

    "It is certain.",
    "It is decidedly so.",
    "Without a doubt.",
    "Yes, definitely.",
    "You may rely on it.",
    "As I see it, yes.",
    "Most likely.",
    "Outlook good.",
    "Yes.",
    "Signs point to yes.",

    "Reply hazy, try again.",
    "Ask again later.",
    "Better not tell you now.",
    "Cannot predict now.",
    "Concentrate and ask again.",

    "Don't count on it.",
    "My reply is no.",
    "My sources say no.",
    "Outlook not so good.",
    "Very doubtful.",
];

fn ball_emoji(idx: usize) -> &'static str {
    match idx {
        0..=9  => emojis::SUCCESS, 
        10..=14 => emojis::CRYSTAL, 
        _      => emojis::ERROR,   
    }
}

pub async fn eight_ball(rest: &RestClient, msg: &Message, question: &str) -> anyhow::Result<()> {
    if question.trim().is_empty() {
        rest.send_message(
            &msg.channel_id,
            &format!("{} Ask a question! Usage: `!8ball will this work?`", emojis::ERROR),
        )
        .await?;
        return Ok(());
    }

    let (idx, answer) = {
        use rand::Rng;
        let i = rand::thread_rng().gen_range(0..RESPONSES.len());
        (i, RESPONSES[i])
    };

    rest.send_message(
        &msg.channel_id,
        &format!(
            "{} **Question:** {}\n**Answer:** {} {}",
            emojis::EIGHT_BALL,
            question.trim(),
            ball_emoji(idx),
            answer
        ),
    )
    .await?;
    Ok(())
}

pub async fn roll(rest: &RestClient, msg: &Message, input: &str) -> anyhow::Result<()> {
    let input = if input.trim().is_empty() {
        "1d6".to_string()
    } else {
        input.trim().to_lowercase()
    };

    let parts: Vec<&str> = input.split('d').collect();
    if parts.len() != 2 {
        rest.send_message(
            &msg.channel_id,
            &format!("{} Invalid format! Try `!roll 2d6`", emojis::ERROR),
        )
        .await?;
        return Ok(());
    }

    let num_dice: u32 = match parts[0].parse() {
        Ok(n) if (1..=100).contains(&n) => n,
        _ => {
            rest.send_message(
                &msg.channel_id,
                &format!("{} Dice count must be 1–100.", emojis::ERROR),
            )
            .await?;
            return Ok(());
        }
    };

    let num_sides: u32 = match parts[1].parse() {
        Ok(n) if (2..=1000).contains(&n) => n,
        _ => {
            rest.send_message(
                &msg.channel_id,
                &format!("{} Sides must be 2–1000.", emojis::ERROR),
            )
            .await?;
            return Ok(());
        }
    };

    let (rolls, total) = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let rolls: Vec<u32> = (0..num_dice).map(|_| rng.gen_range(1..=num_sides)).collect();
        let total: u32 = rolls.iter().sum();
        (rolls, total)
    };

    let msg_text = if num_dice == 1 {
        format!("{} Rolled **{}** on a d{}!", emojis::DICE, total, num_sides)
    } else {
        let rolls_str: Vec<String> = rolls.iter().map(|r| r.to_string()).collect();
        format!(
            "{} **{}d{}** → [{}] = **{}**",
            emojis::DICE,
            num_dice,
            num_sides,
            rolls_str.join(", "),
            total
        )
    };

    rest.send_message(&msg.channel_id, &msg_text).await?;
    Ok(())
}

pub async fn coinflip(rest: &RestClient, msg: &Message) -> anyhow::Result<()> {
    let result = {
        use rand::Rng;
        rand::thread_rng().gen_bool(0.5)
    };

    let text = if result {
        format!("{} **Heads!**", emojis::COIN)
    } else {
        format!("{} **Tails!**", emojis::COIN)
    };

    rest.send_message(&msg.channel_id, &text).await?;
    Ok(())
}
