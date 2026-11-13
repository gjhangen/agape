//! The public side of Agape between people: posts anyone can read, likes, comments, and the
//! profile a person writes about themselves. Teams and players post here too, so the feed is the
//! night told by the people in it. Simulated voices draw from their own hash, never the night's
//! random stream, so a demo with or without the feed plays the same game.

use super::*;
use crate::social::{Post, Profile};
use serde_json::{json, Value};

/// A stable number from a time and a salt: the community's own dice.
fn mix(t: i64, salt: u64) -> u64 {
    let mut h = 1469598103934665603u64 ^ salt.wrapping_mul(0x9E3779B97F4A7C15);
    for b in t.to_le_bytes() { h = (h ^ b as u64).wrapping_mul(1099511628211); }
    h ^ (h >> 29)
}
fn roll(t: i64, salt: u64, p: f64) -> bool { (mix(t, salt) % 10_000) as f64 / 10_000.0 < p }
fn pick<'a>(t: i64, salt: u64, pool: &'a [&'a str]) -> &'a str { pool[(mix(t, salt) % pool.len() as u64) as usize] }

const POST_PRE: &[&str] = &["First game of the season for me. Let's be loud tonight", "Ordered food from my seat before warmups even ended. Never going back",
    "{HOME} by double digits. Calling it now", "Brought my dad to his first {home} game", "Who else gets here early for warmups?", "{star} looked locked in during shootaround"];
const POST_LIVE: &[&str] = &["{STAR} is cooking tonight", "This building is so loud right now", "Best seats I've had all season", "That last run was unreal",
    "Defense is going to win this one", "Runner got my nachos here in six minutes. Six.", "{p2} doesn't get enough credit"];
const POST_SHOW: &[&str] = &["Lights just went down. Here we go", "This crowd is singing every word", "Best sound I've heard in this room", "Merch line was zero people because I ordered from my seat",
    "Waited two years to see this tour", "That opener deserves a headline tour of their own"];
const POST_AFTER: &[&str] = &["What a night. See you at the next one", "Still can't believe that finish", "My voice is gone and I regret nothing", "Already looking at tickets for the next one"];
const COMMENTS: &[&str] = &["facts", "same section, wave!", "let's go", "so true", "this", "we're two rows behind you", "{STAR} for MVP", "loudest it's been all year",
    "agreed", "nah you're right", "say it louder", "best night of the season"];
const COMMENTS_SHOW: &[&str] = &["same", "so good", "this", "we're two rows behind you", "loudest crowd of the tour", "agreed", "say it louder", "best night of the year", "still shaking", "same section, wave!"];
const BIOS_SHOW: &[&str] = &["Front rail or nothing", "Here for the opener, staying for the encore", "Forty shows and counting", "I came for one song and I know which one", "Balcony, dead centre, every time", "Tour shirt from every year"];
const TO_YOU: &[&str] = &["welcome to the crew", "facts", "agreed 100%", "see you at the next one", "which section are you in?", "love this", "same here"];
const BIOS: &[&str] = &["Season ticket holder since the old building", "Here for the nachos and the noise", "Upper deck, loudest section, every night", "Three kids, one jersey each, zero quiet nights",
    "Courtside once. Still talking about it", "Stats nerd. Ask me about {star}'s splits", "Road trips for away games when I can", "First game was 1998. Still my favorite place"];

pub(super) const POST_LIMIT: usize = 280;

impl Night {
    fn add_post(&mut self, who: &str, kind: &str, text: String, t: i64) -> String {
        let id = format!("p{}", self.social.posts.len() + 1);
        self.social.posts.push(Post { id: id.clone(), t, author: who.into(), kind: kind.into(), text, likes: Default::default(), comments: vec![], shown_as: None });
        id
    }

    /// The venue or an act on tonight's bill posts under its own name.
    fn add_named_post(&mut self, name: &str, kind: &str, text: String, t: i64) {
        let id = self.add_post("venue", kind, text, t);
        if let Some(p) = self.social.posts.iter_mut().find(|p| p.id == id) { p.shown_as = Some(name.to_string()); }
    }

    /// A person posts to everyone here.
    pub fn post_as(&mut self, who: &str, text: &str) -> Result<String, String> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > POST_LIMIT { return Err(format!("Write something up to {POST_LIMIT} characters")); }
        let text = moderate(text).map_err(|e| e.replace("in the crowd chat", "in posts"))?;
        let t = self.t;
        Ok(self.add_post(who, "fan", text, t))
    }

    /// Like a post, or take the like back. Returns whether it's liked now.
    pub fn like(&mut self, who: &str, post: &str) -> Result<bool, String> {
        let p = self.social.posts.iter_mut().find(|p| p.id == post).ok_or("That post is gone")?;
        if p.likes.remove(who) { Ok(false) } else { p.likes.insert(who.into()); Ok(true) }
    }

    pub fn comment(&mut self, who: &str, post: &str, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > 200 { return Err("Write a comment up to 200 characters".into()); }
        let text = moderate(text).map_err(|e| e.replace("in the crowd chat", "in comments"))?;
        let t = self.t;
        let p = self.social.posts.iter_mut().find(|p| p.id == post).ok_or("That post is gone")?;
        p.comments.push((t, who.into(), text));
        Ok(())
    }

    /// Your own post can be taken down by you.
    pub fn delete_post(&mut self, who: &str, post: &str) -> Result<(), String> {
        let i = self.social.posts.iter().position(|p| p.id == post).ok_or("That post is gone")?;
        if self.social.posts[i].author != who { return Err("Only the person who posted it can take it down".into()); }
        self.social.posts[i].text.clear(); self.social.posts[i].kind = "removed".into();
        Ok(())
    }

    /// Report a post. The reporter stops seeing it at once; three reports hide it from everyone
    /// until Agape's team has looked. A post the team already kept is not hidden again.
    pub fn report_post(&mut self, who: &str, post: &str, reason: &str) -> Result<(), String> {
        if !matches!(reason, "spam" | "abuse" | "unsafe" | "other") { return Err("Pick what's wrong with it".into()); }
        let p = self.social.posts.iter().find(|p| p.id == post && p.kind != "removed").ok_or("That post is gone")?;
        if p.author == who { return Err("That's your own post. You can delete it.".into()); }
        if self.social.reports.iter().any(|r| r.0 == post && r.1 == who) { return Ok(()); }
        let t = self.t;
        self.social.reports.push((post.into(), who.into(), reason.into(), t));
        Ok(())
    }

    /// Agape's team decides a reported post: leave it up, or take it down.
    pub fn moderate(&mut self, post: &str, remove: bool) -> Result<(), String> {
        let i = self.social.posts.iter().position(|p| p.id == post).ok_or("That post is gone")?;
        if remove { self.social.posts[i].text.clear(); self.social.posts[i].kind = "removed".into(); }
        else { self.social.kept.insert(post.into()); }
        Ok(())
    }

    fn reports_on(&self, post: &str) -> usize { self.social.reports.iter().filter(|r| r.0 == post).count() }

    /// Whether this viewer shouldn't see the post: they reported it, or enough people have and nobody has reviewed it yet.
    fn hidden_from(&self, p: &Post, viewer: &str) -> bool {
        if p.author == viewer { return false; }
        self.social.reports.iter().any(|r| r.0 == p.id && r.1 == viewer) || (!self.social.kept.contains(&p.id) && self.reports_on(&p.id) >= 3)
    }

    /// The posts waiting on Agape's team, most reported first.
    pub fn reported_json(&self) -> Vec<Value> {
        let mut out: Vec<(usize, Value)> = self.social.posts.iter().filter(|p| p.kind != "removed" && !self.social.kept.contains(&p.id)).filter_map(|p| {
            let rs: Vec<&(String, String, String, i64)> = self.social.reports.iter().filter(|r| r.0 == p.id).collect();
            if rs.is_empty() { return None; }
            let mut why: BTreeMap<&str, usize> = BTreeMap::new();
            for r in &rs { *why.entry(r.2.as_str()).or_insert(0) += 1; }
            Some((rs.len(), json!({ "post": p.id, "text": p.text, "author": p.author, "who": p.shown_as.clone().or(self.party_name(&p.author)).unwrap_or_else(|| crate::social::handle(&p.author)),
                "kind": p.kind, "t": p.t, "reports": rs.len(), "why": why, "hidden": rs.len() >= 3, "first": rs.iter().map(|r| r.3).min() })))
        }).collect();
        out.sort_by(|a, b| b.0.cmp(&a.0));
        out.into_iter().map(|x| x.1).collect()
    }

    /// What a person says about themselves: the name others see, a line about them, and their team.
    pub fn set_profile(&mut self, who: &str, name: &str, bio: &str, team: &str) -> Result<(), String> {
        let name: String = name.trim().chars().take(24).collect();
        let bio = bio.trim();
        if bio.chars().count() > 160 { return Err("Keep the line about you to 160 characters".into()); }
        if !team.is_empty() && self.league.team(team).is_none() { return Err("Pick one of the league's teams".into()); }
        let bio = if bio.is_empty() { String::new() } else { moderate(bio).map_err(|_| "Links, phone numbers and selling can't go on a profile".to_string())? };
        if !name.is_empty() { moderate(&name).map_err(|_| "That name can't be used".to_string())?; self.names.insert(who.into(), name); }
        self.social.profiles.insert(who.into(), Profile { bio, team: team.into() });
        Ok(())
    }

    /// The profile a person wrote, or for a simulated fan one that fits them.
    pub fn profile_of(&self, id: &str) -> Profile {
        if let Some(p) = self.social.profiles.get(id) { return p.clone(); }
        if self.cfg.real || id == YOU || !self.fans.iter().any(|f| f.id == id) { return Profile::default(); }
        let h = mix(id.len() as i64, id.bytes().fold(7u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64)));
        let team = if h % 5 == 0 { self.league.tonight.away.clone() } else { self.league.tonight.home.clone() };
        let pool = if self.concert().is_some() { BIOS_SHOW } else { BIOS };
        Profile { bio: self.crowd_words(pool[((h >> 8) % pool.len() as u64) as usize]), team: if self.concert().is_some() { String::new() } else { team } }
    }

    fn sim_fan(&self, t: i64, salt: u64) -> Option<String> {
        let n = sim_fans_n(&self.cfg);
        if n == 0 { return None; }
        Some(self.fans[(mix(t, salt) % n as u64) as usize].id.clone())
    }

    /// Before the doors: the teams, their stars and a few early fans have already posted.
    pub(super) fn seed_posts(&mut self) {
        if self.cfg.real || !self.on("chat") || sim_fans_n(&self.cfg) == 0 { return; }
        let home = self.league.team(&self.league.tonight.home).cloned();
        let away = self.league.team(&self.league.tonight.away).cloned();
        if let Some(c) = self.concert().cloned() {
            let venue = self.world.def.venue.clone();
            self.add_named_post(&venue, "venue", format!("Doors are open at {venue}. Order from your seat with Agape and don't miss a song."), 0);
            if let Some(main) = c.acts.last() { self.add_named_post(main, "act", "Soundcheck done. See you out there tonight.".into(), 0); }
            if c.acts.len() > 1 { let first = c.acts[0].clone(); self.add_named_post(&first, "act", "First on tonight. Come down early, we're bringing the loud ones.".into(), 0); }
        } else {
            if let (Some(h), Some(a)) = (&home, &away) {
                self.add_post(&h.id, "team", format!("Game day. {} host the {} tonight. Order from your seat with Agape and don't miss a play.", h.name, a.short), 0);
                if let Some(s) = h.roster.iter().find(|m| m.star) { self.add_post(&s.id, "player", "Locked in. Bring the noise tonight.".into(), 0); }
                if let Some(s) = a.roster.iter().find(|m| m.star) { self.add_post(&s.id, "player", format!("Big one on the road. Love playing in front of {} fans, they never sit down.", h.short), 0); }
            }
        }
        for k in 0..4u64 {
            let pool = if self.concert().is_some() { POST_SHOW } else { POST_PRE };
            if let Some(f) = self.sim_fan(k as i64, 11) { let text = self.crowd_words(pick(k as i64, 12, pool)); self.add_post(&f, "fan", text, 0); }
        }
        // the night before it starts: likes and a few comments already on them
        for i in 0..self.social.posts.len() {
            let n = 6 + (mix(i as i64, 13) % if self.social.posts[i].kind == "fan" { 20 } else { 140 }) as i64;
            for j in 0..n { if let Some(f) = self.sim_fan(i as i64 * 1000 + j, 14) { self.social.posts[i].likes.insert(f); } }
            for j in 0..(mix(i as i64, 15) % 3) as i64 {
                if let Some(f) = self.sim_fan(i as i64 * 1000 + j, 16) { let c = self.crowd_words(pick(i as i64 * 7 + j, 17, if self.concert().is_some() { COMMENTS_SHOW } else { COMMENTS })); self.social.posts[i].comments.push((0, f, c)); }
            }
        }
    }

    /// Each tick: the teams post what happens, fans post now and then, and the crowd likes and replies.
    pub(super) fn crowd_posts(&mut self, t: i64) {
        if self.cfg.real || !self.on("chat") || sim_fans_n(&self.cfg) == 0 { return; }
        let ph = phase_of(&self.script, t / 60_000).0;
        let was = phase_of(&self.script, (t - TICK).max(0) / 60_000).0;
        let concert = self.concert().is_some();
        let (home, away) = (self.league.tonight.home.clone(), self.league.tonight.away.clone());
        let short = |id: &str| self.league.team(id).map(|g| g.short.clone()).unwrap_or_default();
        if !concert && t > 0 && ph != was {
            let (hs, as_) = self.score;
            let line = match ph {
                "halftime" => Some(format!("Halftime. {} {}, {} {}.", short(&home), hs, short(&away), as_)),
                "postgame" => Some(format!("Final. {} {}, {} {}.", short(&home), hs, short(&away), as_)),
                _ => None,
            };
            if let Some(l) = line { self.add_post(&home.clone(), "team", l, t); }
            if ph == "postgame" {
                let won = if hs >= as_ { &home } else { &away };
                let star = self.league.team(won).and_then(|g| g.roster.iter().find(|m| m.star)).map(|m| m.id.clone());
                if let Some(s) = star { self.add_post(&s, "player", "That's a team win. Thank you for the noise tonight, every one of you.".into(), t + TICK); }
            }
        }
        // a concert: the venue says who's on as each act starts, and the headliner signs off
        if let Some(c) = self.concert().cloned() {
            let m = t / 60_000;
            let period = |m: i64| [45, 76, 122, 153].iter().rposition(|s| m >= *s).map(|i| i as i64 + 1).unwrap_or(0);
            let (now, before) = (period(m), period((t - TICK).max(0) / 60_000));
            if t > 0 && now != before && now >= 1 && now != 4 {
                if let Some(act) = self.act_on(now) { let venue = self.world.def.venue.clone(); self.add_named_post(&venue, "venue", format!("{act} is on now."), t); }
            }
            if t > 0 && ph == "postgame" && was != "postgame" {
                if let Some(main) = c.acts.last() { self.add_named_post(main, "act", "Thank you. Every single one of you. Get home safe.".into(), t); }
            }
        }
        // a big moment: the team it belongs to posts it
        if !concert {
            let fresh = self.feed.iter().rev().take_while(|f| f.t > t - TICK).find(|f| f.kind == "moment").cloned();
            if let Some(f) = fresh {
                let team = self.group_of(&f.team);
                if roll(t, 21, 0.7) { self.add_post(&team, "team", f.text.clone(), t); }
            }
        }
        // fans post now and then
        if t <= self.end_min * 60_000 + 20 * 60_000 && roll(t, 22, 0.035) {
            let pool = if concert { POST_SHOW } else if ph == "pregame" { POST_PRE } else if ph == "postgame" { POST_AFTER } else { POST_LIVE };
            if let Some(f) = self.sim_fan(t, 23) { let text = self.crowd_words(pick(t, 24, pool)); self.add_post(&f, "fan", text, t); }
        }
        // once in a while someone in the crowd reports a fan's post, so Agape's team has something to look at
        if roll(t, 90, 0.004) {
            let fans: Vec<String> = self.social.posts.iter().rev().take(12).filter(|p| p.kind == "fan" && p.author != YOU && !self.live.contains(&p.author)).map(|p| p.id.clone()).collect();
            if !fans.is_empty() {
                let post = fans[(mix(t, 91) % fans.len() as u64) as usize].clone();
                if let Some(f) = self.sim_fan(t, 92) { let _ = self.report_post(&f, &post, pick(t, 93, &["spam", "abuse", "other"])); }
            }
        }
        // the crowd reads recent posts: likes, and now and then a reply
        let n = self.social.posts.len();
        for i in n.saturating_sub(16)..n {
            let p = &self.social.posts[i];
            if p.kind == "removed" { continue; }
            let age = t - p.t;
            let yours = p.author == YOU || self.live.contains(&p.author);
            let heat = if yours { 0.6 } else if p.kind == "fan" { 0.25 } else { 0.8 };
            if age < 15 * 60_000 && roll(t, 30 + i as u64, heat) {
                let k = 1 + mix(t, 40 + i as u64) % if p.kind == "fan" { 2 } else { 9 };
                for j in 0..k as i64 { if let Some(f) = self.sim_fan(t + j, 50 + i as u64) { self.social.posts[i].likes.insert(f); } }
            }
            let p = &self.social.posts[i];
            let replies = p.comments.iter().filter(|c| c.1 != YOU).count();
            let chance = if yours { if replies < 3 && age >= TICK && age < 5 * 60_000 { 0.22 } else { 0.0 } } else if age < 10 * 60_000 { 0.02 } else { 0.0 };
            if chance > 0.0 && roll(t, 60 + i as u64, chance) {
                if let Some(f) = self.sim_fan(t, 70 + i as u64) {
                    let c = self.crowd_words(pick(t, 80 + i as u64, if yours { TO_YOU } else if concert { COMMENTS_SHOW } else { COMMENTS }));
                    self.social.posts[i].comments.push((t, f, c));
                }
            }
        }
    }

    fn post_json(&self, p: &Post, viewer: &str) -> Value {
        let who = |id: &str| if id == viewer { "You".to_string() } else { self.party_name(id).unwrap_or_else(|| crate::social::handle(id)) };
        let author = p.shown_as.clone().unwrap_or_else(|| who(&p.author));
        let team = if p.kind == "team" { self.league.team(&p.author) } else { self.league.member(&p.author).map(|(g, _)| g) };
        let comments: Vec<Value> = p.comments.iter().rev().take(20).rev().map(|c| json!({ "t": c.0, "id": c.1, "who": who(&c.1), "text": c.2, "you": c.1 == viewer })).collect();
        json!({ "id": p.id, "t": p.t, "author": if p.shown_as.is_some() { "" } else { p.author.as_str() }, "who": author, "kind": p.kind, "text": p.text, "you": p.author == viewer,
                "likes": p.likes.len(), "liked": p.likes.contains(viewer), "comments": comments, "replies": p.comments.len(),
                "color": team.map(|g| g.color.clone()), "verified": p.kind != "fan" })
    }

    /// The feed: everything, newest first, or only the people this viewer follows (and themselves).
    pub fn posts_json(&self, viewer: &str, following_only: bool, n: usize) -> Vec<Value> {
        let follows = self.social.follows.get(viewer);
        self.social.posts.iter().rev().filter(|p| p.kind != "removed" && !self.hidden_from(p, viewer))
            .filter(|p| !following_only || p.author == viewer || follows.map_or(false, |f| f.contains(&p.author)))
            .take(n).map(|p| self.post_json(p, viewer)).collect()
    }

    pub fn posts_by(&self, author: &str, viewer: &str, n: usize) -> Vec<Value> {
        self.social.posts.iter().rev().filter(|p| p.author == author && p.kind != "removed" && !self.hidden_from(p, viewer)).take(n).map(|p| self.post_json(p, viewer)).collect()
    }

    /// Everything for the People space: the feed, your profile, and the market in one place.
    pub fn community_json(&self, viewer: &str) -> Value {
        let mine = self.profile_of(viewer);
        let teams: Vec<Value> = self.league.teams.iter().map(|g| json!({ "id": g.id, "name": g.name, "short": g.short, "color": g.color })).collect();
        let mut resale: Vec<&crate::exchange::Offer> = self.trade.offers().filter(|o| o.kind == crate::exchange::Kind::Resale && o.open).collect();
        resale.sort_by_key(|o| o.price);
        let resale: Vec<Value> = resale.into_iter().take(10).map(|o| {
            let ev = o.subject.split(':').next().unwrap_or("");
            let e = self.league.upcoming.iter().find(|e| e.id == ev);
            json!({ "id": o.id, "event": ev, "title": e.map(|e| e.title.clone()), "date": e.map(|e| e.date.clone()), "price": o.price, "section": o.subject.split(':').nth(1),
                    "seats": o.sub, "qty": o.sub.split(',').count(), "mine": o.maker == viewer, "seller": self.party_name(&o.maker) })
        }).collect();
        json!({
            "everyone": self.posts_json(viewer, false, 30),
            "following": self.posts_json(viewer, true, 30),
            "profile": { "name": self.names.get(viewer), "bio": mine.bio, "team": mine.team, "posts": self.social.posts.iter().filter(|p| p.author == viewer && p.kind != "removed").count() },
            "teams": teams,
            "resale": resale,
        })
    }
}
