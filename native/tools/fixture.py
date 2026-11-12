import json, sys, time, base64
out, phase, vphase = sys.argv[1], sys.argv[2], sys.argv[3]
variant = sys.argv[4] if len(sys.argv) > 4 else ""
now = int(time.time() * 1000)
ids = [103, 238, 157, 64, 67, 22, 412, 54, 24, 89]
def stats(i, win, g):
    return {"win": win, "kills": 3 + i + g % 3, "deaths": 2 + (i % 4), "assists": 5 + i, "totalMinionsKilled": 120 + 12 * i, "neutralMinionsKilled": 4, "totalDamageDealtToChampions": 9000 + 1800 * i, "goldEarned": 9000 + 700 * i, "visionScore": 12 + i, "champLevel": 14, "damageDealtToObjectives": 2000 + 300 * i, "largestMultiKill": 2 + (i % 3), "largestCriticalStrike": 300, "totalHeal": 900,
            "item0": 3089, "item1": 3020, "item2": 3157, "item3": 4645, "item4": 0, "item5": 3135, "item6": 3340, "perk0": 8112, "perk1": 8126, "perk2": 8138, "perk3": 8135, "perk4": 8345, "perk5": 8347, "perkPrimaryStyle": 8100, "perkSubStyle": 8300, "statPerk0": 5008, "statPerk1": 5008, "statPerk2": 5002}
def full(gid, n):
    win = (n % 3 == 0 and n >= 3) if variant == "losing" else n % 3 != 2
    parts = []; idents = []
    for i in range(10):
        team = 100 if i < 5 else 200
        parts.append({"participantId": i + 1, "teamId": team, "championId": ids[i], "spell1Id": 4, "spell2Id": 11 if i in (1, 6) else 14, "stats": stats(i, win if team == 100 else not win, n)})
        pu = "me" if i == 0 else f"p{i}"
        idents.append({"participantId": i + 1, "player": {"puuid": pu, "gameName": "Testeur" if i == 0 else f"Joueur{i}", "tagLine": "EUW", "summonerName": ""}})
    teams = [{"teamId": t, "towerKills": 6, "dragonKills": 2, "baronKills": 1, "riftHeraldKills": 1, "hordeKills": 3} for t in (100, 200)]
    return {"gameId": gid, "gameCreation": now - (n * 600_000 if n < 5 else n * 3600_000 * 5), "gameDuration": 1800 if n != 7 else 200, "queueId": 420 if n % 2 else 400, "mapId": 11, "participants": parts, "participantIdentities": idents, "teams": teams}
d = {"lcu:/lol-end-of-game/v1/eog-stats-block": {"gameId": 2000, "gameMode": "ARAM"}, "lcu:/lol-gameflow/v1/gameflow-phase": phase, "lcu:/lol-summoner/v1/current-summoner": {"puuid": "me", "gameName": "Testeur", "tagLine": "EUW", "summonerLevel": 120, "profileIconId": 29, "summonerId": 1}}
d["lcu:/lol-ranked/v1/ranked-stats/me"] = {"queueMap": {"RANKED_SOLO_5x5": {"tier": "GOLD", "division": "II", "leaguePoints": 40, "wins": 50, "losses": 45}, "RANKED_TFT": {"tier": "SILVER", "division": "I", "leaguePoints": 12, "wins": 5, "losses": 30}}}
d["lcu:/lol-game-queues/v1/queues"] = [{"id": 420, "description": "Classé Solo/Duo", "name": ""}, {"id": 400, "description": "Draft", "name": ""}, {"id": 450, "description": "ARAM", "name": ""}]
games = []
for n in range(20):
    f = full(1000 + n, n); d[f"lcu:/lol-match-history/v1/games/{1000 + n}"] = f
    f2 = dict(f); f2["participants"] = [f["participants"][0]]; f2["participantIdentities"] = [f["participantIdentities"][0]]; games.append(f2)
    d[f"lcu:/lol-match-history/v1/game-timelines/{1000 + n}"] = {"frames": [{"participantFrames": {str(i + 1): {"minionsKilled": m * (6 + i % 3), "jungleMinionsKilled": 0, "totalGold": 500 + m * 380 + i * 8, "xp": m * 420} for i in range(10)}} for m in range(24)]}
aram = full(2000, 0)
aram.update({"queueId": 450, "mapId": 12, "gameDuration": 596})
aram["participants"][0]["stats"].update({"totalHeal": 1932, "timeCCingOthers": 36, "totalDamageTaken": 21000})
d["lcu:/lol-match-history/v1/games/2000"] = aram
for count in (19, 99):
    d[f"lcu:/lol-match-history/v1/products/lol/me/matches?begIndex=0&endIndex={count}"] = {"games": {"games": games}}
tft = [{"metadata": {"match_id": f"T{i}"}, "json": {"game_datetime": now - i * 7200_000, "game_length": 1800.5, "queue_id": 1100, "participants": [{"puuid": "me", "placement": 1 + i % 8, "level": 8, "units": [{"characterId": "TFT15_Ahri", "tier": 2, "rarity": 2}], "traits": [{"name": "TFT15_Mage", "num_units": 3, "style": 2}]}]}} for i in range(20)]
d["lcu:/lol-match-history/v1/products/tft/me/matches?begin=0&count=20"] = {"games": tft}
d["lcu:/lol-match-history/v1/products/tft/me/matches?begin=20&count=20"] = {"games": []}
for i in range(1, 10):
    d[f"lcu:/lol-summoner/v2/summoners/puuid/p{i}"] = {"puuid": f"p{i}", "gameName": f"Joueur{i}", "tagLine": "EUW", "summonerLevel": 50, "profileIconId": 29}
    d[f"lcu:/lol-ranked/v1/ranked-stats/p{i}"] = {"queueMap": {"RANKED_SOLO_5x5": {"tier": "PLATINUM", "division": "III", "leaguePoints": 10, "wins": 10, "losses": 5}}}
    d[f"lcu:/lol-match-history/v1/products/lol/p{i}/matches?begIndex=0&endIndex=19"] = {"games": {"games": [games[1], games[2]] if i in (1, 2) else [dict(g, gameId=g["gameId"] + 1000 * i) for g in games]}}
d["lcu:/lol-summoner/v2/summoners/puuid/me"] = d["lcu:/lol-summoner/v1/current-summoner"]
d["lcu:/lol-champ-select/v1/session"] = {"localPlayerCellId": 0, "myTeam": [{"cellId": i, "puuid": "me" if i == 0 else f"p{i}", "championId": ids[i], "assignedPosition": ["middle", "jungle", "top", "bottom", "utility"][i], "gameName": ""} for i in range(5)], "theirTeam": [{"cellId": 5 + i, "puuid": "", "championId": ids[5 + i], "assignedPosition": "", "gameName": ""} for i in range(5)]}
d["lcu:/lol-champ-select/v1/current-champion"] = 103
d["lcu:/lol-gameflow/v1/session"] = {"map": {"id": 11}}
J, O = "add6443a-41bd-e414-f6ad-e58d267f4e95", "eb93336a-449b-9c1b-0a54-a891f7921d69"
priv = base64.b64encode(json.dumps({"matchPresenceData": {"sessionLoopState": vphase}, "partyPresenceData": {"partyId": "party1"}}).encode()).decode()
d["valo:session:"] = {"puuid": "vme", "region": "eu", "shard": "eu"}
d["valo:local:/chat/v4/presences"] = {"presences": [{"puuid": "vme", "product": "valorant", "private": priv}, {"puuid": "v1", "product": "valorant", "private": priv}]}
def vmatch(i):
    pl = []
    for k in range(10):
        sub = "vme" if k == 0 else f"v{k}"
        pl.append({"subject": sub, "partyId": "groupe-essai" if k in (0, 1) else f"seul-{k}", "gameName": "" if k else "TesteurV", "tagLine": "" if k else "EUW", "teamId": "Blue" if k < 5 else "Red", "characterId": J if k % 2 == 0 else O, "competitiveTier": 15, "stats": {"score": 4000 + 200 * k, "roundsPlayed": 20, "kills": 18 - k, "deaths": 10 + k, "assists": 4}})
    rounds = [{"roundNum": r, "winningTeam": "Blue" if r % 3 else "Red", "roundResult": "Eliminated", "bombPlanter": "vme" if r == 2 else None, "plantSite": "A" if r == 2 else None, "plantRoundTime": 40000 if r == 2 else None,
               "playerStats": [{"subject": "vme", "kills": [{"killer": "vme", "victim": "v5", "roundTime": 15000, "finishingDamage": {"damageType": "Weapon", "damageItem": "9C82E19D-4575-0200-1A81-3EACF00CF872"}}, {"killer": "vme", "victim": "v6", "roundTime": 16000, "finishingDamage": {"damageType": "Ability", "damageItem": "x"}}, {"killer": "vme", "victim": "v7", "roundTime": 17000, "finishingDamage": {"damageType": "Melee", "damageItem": ""}}], "damage": [{"damage": 300, "headshots": 3, "bodyshots": 6, "legshots": 1}], "economy": {"loadoutValue": 3900}}, {"subject": "v1", "kills": [], "damage": [], "economy": {"loadoutValue": 3900}}]} for r in range(20)]
    return {"matchInfo": {"matchId": f"m{i}", "mapId": "/Game/Maps/Ascent/Ascent", "gameLengthMillis": 2100000, "gameStartMillis": now - i * 3600_000 * 6, "queueID": "competitive"}, "players": pl, "teams": [{"teamId": "Blue", "won": True, "roundsWon": 13, "roundsPlayed": 20}, {"teamId": "Red", "won": False, "roundsWon": 7, "roundsPlayed": 20}], "roundResults": rounds}
hist = [{"MatchID": f"m{i}"} for i in range(20)]
for pu in ["vme"] + [f"v{k}" for k in range(1, 10)]:
    d[f"valo:pd:/match-history/v1/history/{pu}?startIndex=0&endIndex=20"] = {"History": hist}
    for queue in ("competitive", "unrated", "swiftplay", "deathmatch", "premier"):
        d[f"valo:pd:/match-history/v1/history/{pu}?startIndex=0&endIndex=20&queue={queue}"] = {"History": hist if queue == "competitive" else []}
    d[f"valo:pd:/mmr/v1/players/{pu}"] = {"QueueSkills": {"competitive": {"SeasonalInfoBySeasonID": {"act": {"CompetitiveTier": 15, "RankedRating": 44, "NumberOfWinsWithPlacements": 12, "NumberOfGames": 20, "WinsByTier": {"15": 3, "18": 1}}}}}}
    d[f"valo:pd:/mmr/v1/players/{pu}/competitiveupdates?startIndex=0&endIndex=20&queue=competitive"] = {"Matches": [{"MatchID": f"m{i}", "RankedRatingEarned": 18 if i % 2 else -9, "TierBeforeUpdate": 15, "TierAfterUpdate": 15, "IsPlacementMatch": False} for i in range(20)]}
for i in range(20):
    d[f"valo:pd:/match-details/v1/matches/m{i}"] = vmatch(i)
d["valo:shared:/content-service/v3/content"] = {"Seasons": [{"ID": "act", "Type": "act", "IsActive": True}]}
d["valo:pd:/name-service/v2/players"] = [{"Subject": "vme", "GameName": "TesteurV", "TagLine": "EUW"}] + [{"Subject": f"v{k}", "GameName": f"Valo{k}", "TagLine": "000"} for k in range(1, 10)]
d["valo:glz:/pregame/v1/players/vme"] = {"MatchID": "live"}
d["valo:glz:/pregame/v1/matches/live"] = {"AllyTeam": {"TeamID": "Blue", "Players": [{"Subject": "vme" if k == 0 else f"v{k}", "CharacterID": J, "CharacterSelectionState": "locked" if k < 3 else "selected", "PlayerIdentity": {"AccountLevel": 80, "Incognito": k == 4}} for k in range(5)]}, "MapID": "/Game/Maps/Ascent/Ascent", "QueueID": "competitive"}
d["valo:glz:/parties/v1/players/vme"] = {"CurrentPartyID": "party1"}
d["valo:glz:/parties/v1/parties/party1"] = {"Members": [{"Subject": "vme"}, {"Subject": "v1"}]}
json.dump(d, open(out, "w"))
