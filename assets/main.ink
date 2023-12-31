VAR intersections = 0
-> main


=== main
Howdy partner, do you want to learn how to untangle a map?
+ Affirmative!
+ More information requested?
Aw shucks partner, you probably don't know what a map is, an why it needs untanglin'.
Well now let me tell ya, a map is a bunch of big dots. An it needs untanglin' so that the supervisors can do them their jobs.
+ + Information requested regarding supervisors?
  Aw, never mind all that now, or we'll be here all day. An ya let the supervisors do their thing, they'll let ya do yours.
+ + Proceed...
-
# SIZE 4 5 # RESET
Righty ho. You should be seeing some spots and lines now.
+ Confirmed!
+ Proceed.
- -> try_reset

= try_reset
{intersections > 0:
-> good_map
}
{Tarnation, that map ain't have anything to untangle!|{&Again, nothing to untangle!|Nothing to untangle...}} Let me getcha a new one.
+ Proceed. # RESET
-> try_reset
+ Negatory! Current map is adequate.
-
{Aw nonsense, I can't ask you to untangle a map that ain't even tangled, it wouldn't be right.|Aw no, that ain't right.} Give me two ticks. # RESET
+ Interval elapsed!
  I'm doing my best here, just give me a bit of patience. And... got it!
+ Proceed.
  See now, there we go.
- -> try_reset

= good_map
Alrighty partner. You've got yerself a map! Yer gonna see a big pack a dots, and there'll be some lines that connect them to each other.
+ Visual match confirmed.
  Yer learning quick, ain't ya?
+ Description matches existing knowledge of map...
  Shucks, I'm talking through the obvious, aren't I?

+ Requesting additional context regarding dots?
  They're the round things... Aw yis, yer meaning the context around what's a dot doing in a map. I think they're machinery, and the supervisors gotta keep them in line, ain't it so.

- Arright. <>
- (do) Yer gonna need to move the dots around, so that none of the connection lines are crossing over each other.
+ Complete! # SOLVED
+ Map remains tangled...
Make sure you click on one of the dots and put it in a different spot. You'll get there.
-> do
- -> good_map
-> DONE
