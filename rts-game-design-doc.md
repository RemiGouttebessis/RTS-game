# RTS/Grand Strategy Game — Design Document

**Core pitch:** A huge, cylindrical, non-tile-based world with up to 32 nations (players + AI), sessions targeting 4–7 hours, simulated across a settlement hierarchy from hamlet to empire, with full economic, technological, diplomatic, and espionage layers alongside military conquest.

---

## 1. World & Geography

- **Cylindrical wrap.** The world wraps east-west (like a scroll rolled into a tube), with distinct poles/edges north-south. This gives a "huge, endless" feel for frontline warfare and long supply lines without the visual/UX cost of a full sphere.
- **Continuous, non-tile terrain.** Heightmap/mesh-based terrain rather than a grid of tiles. Settlements, roads, farmland, and borders are placed organically rather than snapped to a grid — this is what makes "huge world" actually *feel* huge rather than feeling like a big spreadsheet.
- **Civilization-style world generation**, adapted to continuous space: procedural continents/landmasses, climate bands (driven by latitude, since the cylinder still has poles), rivers flowing from high to low elevation, and resource distribution (metals, fertile land, strategic resources) seeded by biome + noise. World gen should produce a world worth 4–7 hours of play — meaning enough contested land and resource variety that 32 nations aren't just spreading out uncontested for the first hour.
- **Regions as the mid-scale geographic unit.** Below "nation," the world is divided into regions — contiguous areas that contain multiple settlements and have their own terrain character (a fertile river region, a mountainous mining region, a coastal trade region). Regions are the natural unit for a lot of systems below: taxation, unrest, supply lines, AI strategic planning.

## 2. Settlement Hierarchy

Community → Village → Town → City → Region → Nation → Empire.

- **Each tier is a real entity with its own stats**, not just a population number: population, happiness/unrest, buildings, defenses, resource output, garrison. Lower tiers (community, village) are lightweight — mostly population and a resource output. Town and above start accumulating buildings, specialization, and defenses.
- **Growth and promotion.** A community grows into a village, a village into a town, a town into a city, based on population thresholds and player investment (infrastructure, not just organic growth) — this gives the player a meaningful "where do I invest to grow my next city" decision rather than pure auto-growth.
- **Player-editable layout.** Once a settlement reaches town size or so, the player can reshape its internal layout — where the market goes, where walls run, where the garrison sits, which roads connect to which gates. This ties directly into sieges (below): a city's actual layout determines siege dynamics, not an abstract HP bar.
- **Specialization.** Settlements can lean into a role — a mining town, a trade city, a fortress town, an agricultural region — with mechanical bonuses and trade-offs (a fortress town produces less wealth but is much harder to siege). This gives regions distinct identity and gives conquest actual strategic texture (you *want* the trade city, you *fear* the fortress town).

## 3. Population & Civics

This is the "power over population" layer — how you govern, not just how you grow.

- **Population as a real resource with needs**, not just a growth multiplier: food, safety, and a "contentment"/happiness stat that responds to taxation, war exhaustion, civic policy, and (if you want depth) culture/religion friction between your population and newly conquered populations.
- **Civics as a policy tree**, not a tech tree — things like: centralized vs. feudal authority (fast top-down control vs. slower but more resilient local autonomy), slavery/serfdom vs. free labor (production tradeoffs with unrest/diplomatic consequences), religious tolerance vs. state religion, conscription policy. Civics should be *switchable* mid-game with a cost (unrest spike, gold cost, cooldown) rather than a one-time pick, so it's a lever the player actively manages rather than a build order.
- **Governors/local authority.** At city+ scale, consider letting the player delegate settlement management to an AI "governor" with a configurable priority (growth, military, wealth) — at 32-nation, multi-hour scale, micromanaging every settlement isn't fun; the interesting decisions are *which* settlements you personally steer versus delegate.
- **Unrest and rebellion as a real threat**, especially for newly conquered settlements — an occupied city with unhappy population should be a genuine liability (garrison drain, production penalty, chance of revolt) rather than an instant free city. This makes conquest a real cost-benefit decision, not just "biggest army wins."

## 4. Science & Technology

- **Tech tree gated by civics as much as by tech itself** — some technologies require a civic prerequisite (you can't research advanced siege engineering under certain feudal civics, say), so the two systems interact rather than sitting side by side.
- **Research generation from settlements**, likely tied to population + specialization (a city with a "scholar" specialization or library building produces more) rather than a flat national number — this keeps the settlement layout/specialization decisions relevant to tech too.
- **Branches worth having, given the scope:**
  - *Military* — unit upgrades, siege equipment, fortification tech.
  - *Economy* — production efficiency, trade range/value, resource extraction.
  - *Civic/administration* — settlement management range, unrest reduction, governor effectiveness.
  - *Espionage/statecraft* — spy effectiveness, counter-intel, diplomatic reach.
- **Diminishing/scaling costs at 32-nation scale.** With that many nations, tech cost curves need to account for snowballing — either cost scaling with number of settlements/empire size, or a rubber-band mechanic (smaller nations research cheaper) so mid-game isn't already decided by hour one.

## 5. Espionage

- **Spies as agents**, not abstract percentage rolls — an actual entity (or lightweight agent object, doesn't need full unit-level sim) assigned to a target nation/settlement, with a travel time, a detection risk, and a mission type: sabotage (damage production/defenses), steal tech, incite unrest, assassinate a leader/governor, gather intelligence (reveal enemy army positions/strength — valuable at this scale where you can't see everything).
- **Counter-espionage as a real settlement/nation investment** — garrisons or dedicated buildings that raise detection chance, so espionage is a cat-and-mouse system rather than a free action.
- **Intelligence-gathering matters more than usual here** because of the sheer world scale — with 32 nations and a huge cylindrical map, players (and AI) genuinely can't see everything happening; spies become one of the main tools for reducing that fog, alongside traditional scouting.

## 6. Wealth, Trade & Taxes

- **Trade as a network, not a stat.** Trade routes between settlements (and between nations, if at peace/treaty) generate wealth based on distance, road/infrastructure quality, and goods complementarity (a region with surplus grain trading to a region with a food deficit). This gives roads and infrastructure investment real weight, and gives war a real economic cost (cutting enemy trade routes is a valid military-economic strategy, ties into "frontline" from the military side).
- **Taxation as a dial with consequences**, not a flat percentage: higher tax = more wealth but rising unrest, scaling by civic type (a centralized civic tolerates higher tax better than a feudal one, say). Tax rate could be set nationally or per-region, letting players squeeze conquered/frontier regions harder than their core.
- **Wealth spent, not just accumulated** — upkeep for army, buildings, civic policies, bribery (ties to diplomacy/espionage below), and infrastructure. Avoid a system where gold just piles up with nothing meaningful to spend it on at the mid-late game; that's where 4-7-hour sessions tend to go stale.
- **Black market/corruption** as an optional deeper layer — distance from capital or low administrative tech increasing "leakage" (lost tax revenue), giving another use for the civic/administration tech branch and another reason regional governors matter.

## 7. Diplomacy

- **Core diplomatic actions:** alliances, non-aggression pacts, trade agreements, vassalage/tributary status (relevant given the empire-scale ambition), war declarations with justification/casus belli (adds a soft constraint so 32 nations don't dogpile chaotically), royal marriages or equivalent soft-power tools if you want CK-style flavor.
- **AI diplomacy needs to be legible at 32-nation scale.** With that many actors, players need a way to actually parse the diplomatic web — a relations overview screen, not just discovering alliances when two AI nations suddenly gang up. Consider a visible (if imperfect) "threat/opinion" readout per nation.
- **Diplomacy interacts with espionage and economy directly** — trade agreements should meaningfully boost the trade-network wealth above; spying on/sabotaging an ally should carry a real diplomatic penalty if discovered, so espionage has actual risk beyond "did the mission succeed."
- **Power/influence as a soft currency**, usable for diplomatic actions (proposing alliances, pressuring weaker nations, contesting territory claims) — separate from gold, so diplomatic and economic strength aren't the exact same axis, giving nations different viable strategies (a wealthy trade empire vs. a diplomatically dominant hegemon vs. a military conqueror).

## 8. Military, Sieges & Frontlines (carried over + tied into the above)

- **Frontline as emergent**, computed from contested regions/spatial ownership rather than a hand-tracked object — visualized for the player as a dynamic line/zone rather than a fixed border.
- **Siege as a settlement state**, targeting the actual layout (walls, gates) the player designed in section 2 — breach points matter, garrison composition matters, and a besieged city's population unrest (section 3) can tip a siege via revolt as much as combat can.
- **War exhaustion and supply lines** tie directly into wealth/trade (section 6) — a long siege or overextended campaign should visibly strain the besieging nation's economy, not just be "free" until the battle is won or lost. This is what keeps a 4-7 hour session dramatically paced rather than a slow grind to an inevitable outcome.

---

## Open Design Questions

- Victory conditions at this scale — conquest-only, or also score/economic/diplomatic win paths given the multi-hour session length?
- How much of the 32 nations are real-time simulated at full fidelity vs. abstracted (ties back to the LOD simulation note from the architecture discussion)?
- Player count — is this single-player-vs-31-AI, or does it support multiple human players, and if so how does that change pacing for a 4-7 hour session?
- Espionage and diplomacy both want a "nation overview" UI — worth designing that as one unified screen early, since several systems above feed into it.
