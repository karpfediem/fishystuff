{ runCommandLocal, filteredWaypointsSrc }:

runCommandLocal "bot-waypoints" { } ''
  mkdir -p $out/bdo-fish-waypoints
  cp -r ${filteredWaypointsSrc}/Bookmark $out/bdo-fish-waypoints/
  cp -r ${filteredWaypointsSrc}/FishBookmark $out/bdo-fish-waypoints/
''
