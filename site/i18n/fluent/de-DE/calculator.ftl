# Core calculator UI
calculator.loading = Rechner wird geladen...

calculator.timespan.unit.minute.one = Minute
calculator.timespan.unit.minute.other = Minuten
calculator.timespan.unit.hour.one = Stunde
calculator.timespan.unit.hour.other = Stunden
calculator.timespan.unit.day.one = Tag
calculator.timespan.unit.day.other = Tage
calculator.timespan.unit.week.one = Woche
calculator.timespan.unit.week.other = Wochen

calculator.resource.exhausted = Erschöpft
calculator.resource.low = Niedrig
calculator.resource.average = Mittel
calculator.resource.abundant = Reichlich

calculator.timeline.bite_time = Bisszeit
calculator.timeline.auto_fishing_time = Auto-Fishing-Zeit
calculator.timeline.catch_time = Fangzeit
calculator.timeline.time_saved = Gesparte Zeit

calculator.share.active_lead = Aktiv | 
calculator.share.afr_lead = {$afr} AFR | 
calculator.share.link = [Fishy Stuff-Rechner-Preset | {$lead}{$item_drr} Gegenstands-DRR | {$zone}]({$url})

calculator.toast.preset_url_copied = Preset-URL kopiert.
calculator.toast.share_copied = Freigabetext kopiert.
calculator.toast.cleared = Rechner zurückgesetzt.
calculator.toast.layout_reset = Layout zurückgesetzt.

calculator.layout_presets.title = Layout-Vorlagen
calculator.layout_presets.current = Aktuelles Layout
calculator.layout_presets.default = Standard
calculator.layout_presets.default_name = Layout {$index}

calculator.title.casts_average = Durchschnittliche Würfe ({$timespan})
calculator.title.durability_loss_average = Durchschnittlicher Haltbarkeitsverlust ({$timespan})
calculator.title.expected_catches = Erwartete Fänge ({$timespan})
calculator.title.expected_profit = Erwarteter Gewinn ({$timespan})
calculator.title.bite_time = Bisszeit: {$seconds}s ({$percent}%)
calculator.title.auto_fishing_time = Auto-Fishing-Zeit: {$seconds}s ({$percent}%)
calculator.title.catch_time = Fangzeit: {$seconds}s ({$percent}%)
calculator.title.unoptimized_time = Durchschnittliche unoptimierte Zeit: {$seconds}s ({$percent}%)

# Breakdown labels used in visible overlays and group fallbacks
calculator.breakdown.kind.computed_stat = Berechneter Wert
calculator.breakdown.label.silver_share = Silberanteil
calculator.breakdown.label.unassigned = Nicht zugewiesen

# Personal overlay panel
calculator.overlay.title = Persönliches Overlay
calculator.overlay.current_zone_proposal = Aktueller Zonenvorschlag
calculator.overlay.description = Diese Änderungen bleiben nur im Browser gespeichert. Exportiere das Overlay-JSON, wenn du einen Vorschlag einreichen willst, der später in einen Dolt-Merge-Request umgewandelt werden kann.

calculator.overlay.group.prize = Preisfisch
calculator.overlay.group.rare = Selten
calculator.overlay.group.high_quality = Hochwertig
calculator.overlay.group.general = Allgemein
calculator.overlay.group.trash = Schrott
calculator.overlay.group.unassigned = Nicht zugewiesen

calculator.overlay.change.scope.global_price = Globaler Preis
calculator.overlay.change.group_label = Gruppe {$group}
calculator.overlay.change.detail.raw = roh {$percent}
calculator.overlay.change.detail.base_price = Basispreis {$silver}
calculator.overlay.change.detail.customized = angepasst
calculator.overlay.change.detail.removed_from_zone_mix = aus dem Zonenmix entfernt
calculator.overlay.change.detail.forced_into_zone_mix = in den Zonenmix erzwungen
calculator.overlay.change.detail.added_to_zone = zur Zone hinzugefügt
calculator.overlay.change.detail.removed_from_zone = aus der Zone entfernt
calculator.overlay.change.detail.group = Gruppe {$group}

calculator.overlay.badge.facts = Fakten
calculator.overlay.badge.price = Preis

calculator.overlay.item.fallback_label = Gegenstand {$id}
calculator.overlay.item.id = ID {$id}
calculator.overlay.item.default_raw = Roh {$percent}
calculator.overlay.item.included = Enthalten
calculator.overlay.item.raw_percent = Roh-%
calculator.overlay.item.normalized = normalisiert
calculator.overlay.item.base_silver = Basis-Silber
calculator.overlay.item.default_included = in den Quell-Standardwerten enthalten
calculator.overlay.item.default_absent = in den Quell-Standardwerten nicht vorhanden

calculator.overlay.group.raw_base_percent = Roh-Basis-%
calculator.overlay.group.detail.effective_raw_before_normalization = effektives Rohgewicht {$percent} vor der Normalisierung
calculator.overlay.group.detail.raw_plus_bonus_before_normalization = roh {$raw} + Bonus {$bonus} vor der Normalisierung

calculator.overlay.action.import_json = JSON importieren
calculator.overlay.action.export_json = JSON exportieren
calculator.overlay.action.reset_zone = Zone zurücksetzen
calculator.overlay.action.reset_all = Alles zurücksetzen
calculator.overlay.action.restore = Wiederherstellen
calculator.overlay.action.add_item = Overlay-Eintrag hinzufügen

calculator.overlay.section.zone_groups = Zonengruppen
calculator.overlay.section.zone_groups_help = Bearbeite nur die Roh-Basisraten der Gruppen. Bonus- und normalisierte Werte sind schreibgeschützte Rechnerausgaben.
calculator.overlay.section.zone_groups_notice = Der normalisierte Anteil verwendet das effektive Rohgewicht, nicht nur den Rohwert. Der Rechner addiert zuerst jeden aufgelaufenen Gruppenbonus auf das Roh-% und normalisiert dann alle aktiven Gruppen auf 100 %.
calculator.overlay.section.current_changes = Aktuelle Änderungen
calculator.overlay.section.current_changes_count.one = {$count} aktiver Overlay-Eintrag über Zonen und Preise hinweg.
calculator.overlay.section.current_changes_count.other = {$count} aktive Overlay-Einträge über Zonen und Preise hinweg.
calculator.overlay.section.no_changes = Noch keine persönlichen Overlay-Änderungen. Änderungen bleiben lokal, bis du das JSON exportierst und an die Maintainer schickst.
calculator.overlay.section.zone_items = Zoneneinträge
calculator.overlay.section.zone_items_help = Ändere Zonenzugehörigkeit, rohe Gegenstandsrate oder lokale Gegenstandspreise für die aktuelle Rechnerzone. Normalisierte Ergebnisse bleiben schreibgeschützt.
calculator.overlay.section.add_item = Eintrag hinzufügen
calculator.overlay.section.add_item_help = Verwende das für Gegenstände, die in den aktuellen Zonen-Standardwerten fehlen. Hinzugefügte Zeilen existieren nur im Overlay, bis sie eingereicht und in den Quelldatensatz übernommen werden.

calculator.overlay.column.group = Gruppe
calculator.overlay.column.default = Standard
calculator.overlay.column.present = Vorhanden
calculator.overlay.column.raw_percent = Roh-%
calculator.overlay.column.bonus = Bonus
calculator.overlay.column.normalized = Normalisiert
calculator.overlay.column.item = Gegenstand
calculator.overlay.column.state = Status
calculator.overlay.column.base_price = Basispreis

calculator.overlay.field.item_id = Gegenstands-ID
calculator.overlay.field.label = Bezeichnung
calculator.overlay.field.group = Gruppe
calculator.overlay.field.raw_percent = Roh-%
calculator.overlay.field.base_price = Basispreis
calculator.overlay.field.grade = Qualität
calculator.overlay.field.fish = Fisch
calculator.overlay.field.is_fish = Ist Fisch

calculator.overlay.placeholder.item_name = Fisch- oder Gegenstandsname

calculator.overlay.option.auto = Automatisch

calculator.overlay.toast.imported = Overlay-JSON importiert.
calculator.overlay.toast.import_failed = Overlay-JSON-Import fehlgeschlagen.
calculator.overlay.toast.add_item_missing = Das Formular zum Hinzufügen braucht eine Gegenstands-ID und eine Bezeichnung.
calculator.overlay.toast.downloaded = Overlay-JSON heruntergeladen.

calculator.overlay.error.import_unavailable_browser = Overlay-Import ist in diesem Browser nicht verfügbar.
calculator.overlay.error.read_failed = Overlay-JSON konnte nicht gelesen werden.
calculator.overlay.error.import_unavailable = Overlay-Import ist nicht verfügbar.

# Server-rendered calculator UI
calculator.server.option.none = Keine

calculator.server.result.no_matching_options = Keine passenden Optionen
calculator.server.result.no_matching_zones = Keine passenden Zonen
calculator.server.result.more_available = Weitere Ergebnisse laden
calculator.server.result.selected = Ausgewählt
calculator.server.result.added = Hinzugefügt

calculator.server.action.remove = {$label} entfernen
calculator.server.action.copy_url = URL kopieren
calculator.server.action.copy_share = Freigabe kopieren
calculator.server.action.reset_layout = Layout zurücksetzen
calculator.server.action.clear = Leeren
calculator.server.action.pin_section = {$label} anheften
calculator.server.action.unpin_section = {$label} lösen
calculator.server.action.drag_section = {$label} ziehen
calculator.server.action.drag_section_generic = Abschnitt ziehen
calculator.server.action.drag_unpinned_slot = Slot für nicht angehefteten Abschnitt ziehen
calculator.server.action.pin_dropzone_title = Hier ablegen zum Anheften
calculator.server.action.pin_dropzone_detail = Angeheftete Abschnitte bleiben über dem ausgewählten Tab.
calculator.server.action.unpinned_dropzone_title = Hier ablegen, um den Slot für nicht angeheftete Abschnitte zu verschieben
calculator.server.action.unpinned_dropzone_detail = Künftige nicht angeheftete Tabs erscheinen hier.
calculator.server.action.move_pinned_section_up = {$label} nach oben
calculator.server.action.move_pinned_section_down = {$label} nach unten

calculator.server.toggle.active_fishing = Aktives Fischen
calculator.server.toggle.debug = Debug

calculator.server.search.zones = Zonen suchen
calculator.server.search.fishing_levels = Angelstufen suchen
calculator.server.search.session_units = Sitzungseinheiten suchen
calculator.server.search.lifeskill_levels = Lebensskill-Stufen suchen
calculator.server.search.rods = Ruten suchen
calculator.server.search.floats = Posen suchen
calculator.server.search.chairs = Stühle suchen
calculator.server.search.lightstone_sets = Lichtstein-Sets suchen
calculator.server.search.backpacks = Rucksäcke suchen
calculator.server.search.foods = Nahrung nach Name oder Effekt suchen
calculator.server.search.buffs = Buffs nach Name oder Effekt suchen
calculator.server.search.trade_levels = Handelsstufen suchen
calculator.server.search.loot_rows = Beutezeilen an diesem Spot suchen
calculator.server.search.pet_tiers = Haustierstufen suchen
calculator.server.search.pet_specials = Haustier-Spezialisierungen suchen
calculator.server.search.pet_talents = Haustier-Talente suchen
calculator.server.search.pets = Haustiere suchen

calculator.server.section.zone = Zone
calculator.server.section.bite_time = Bisszeit
calculator.server.section.catch_time = Fangzeit
calculator.server.search.pets = Haustiere suchen
calculator.server.section.session = Sitzung
calculator.server.section.gear = Ausrüstung
calculator.server.section.pets = Haustiere
calculator.server.section.overlay_proposal = Overlay-Vorschlag
calculator.server.section.distribution = Verteilung
calculator.server.section.loot = Beute
calculator.server.section.trade = Handel

calculator.server.field.fishing_level = Angelstufe
calculator.server.field.fishing_resources = Angelressourcen
calculator.server.field.active = Aktiv
calculator.server.field.afk = AFK
calculator.server.field.amount = Menge
calculator.server.field.unit = Einheit
calculator.server.field.lifeskill_level = Lebensskill-Stufe
calculator.server.field.fishing_rod = Angelrute
calculator.server.field.brand = Brand
calculator.server.field.float = Pose
calculator.server.field.chair = Stuhl
calculator.server.field.lightstone_set = Lichtstein-Set
calculator.server.field.backpack = Rucksack
calculator.server.field.outfit = Outfit
calculator.server.field.food = Nahrung
calculator.server.field.buffs = Buffs
calculator.server.field.pet = Haustier
calculator.server.field.tier = Stufe
calculator.server.field.special = Spezial
calculator.server.field.talent = Talent
calculator.server.field.skills = Fähigkeiten
calculator.server.field.pet = Haustier
calculator.server.field.mastery = Meisterschaft
calculator.server.field.target_fish = Zielfisch / Beuteobjekt
calculator.server.field.target_amount = Zielmenge
calculator.server.field.pmf_max_count = PMF-Maximalanzahl
calculator.server.field.trade_level = Handelsstufe
calculator.server.field.distance_bonus = Distanzbonus
calculator.server.field.trade_price_curve = Handelspreiskurve
calculator.server.field.discard_grade = Fische bis Qualität verwerfen

calculator.server.stat.seconds = Sekunden
calculator.server.stat.average_total_fishing_time = Durchschnittliche Gesamtangelzeit
calculator.server.stat.average_bite_time = Durchschnittliche Bisszeit
calculator.server.stat.min = Min.
calculator.server.stat.average = Durchschnitt
calculator.server.stat.max = Max.
calculator.server.stat.effective_min = Effektives Minimum
calculator.server.stat.effective_average = Effektiver Durchschnitt
calculator.server.stat.effective_max = Effektives Maximum
calculator.server.stat.auto_fishing_time_aft = Auto-Fishing-Zeit (AFT)
calculator.server.stat.auto_fishing_time_reduction_afr = Auto-Fishing-Zeit-Reduktion (AFR)
calculator.server.stat.item_drr = Gegenstands-DRR
calculator.server.stat.chance_to_consume_durability = Wahrscheinlichkeit für Haltbarkeitsverlust
calculator.server.stat.raw_prize_catch_rate = Rohe Preisfisch-Fangrate
calculator.server.stat.expected_catches = Erwartete Fänge
calculator.server.stat.expected_catches_per_hour = Erwartete Fänge / Stunde
calculator.server.stat.expected_profit = Erwarteter Gewinn
calculator.server.stat.profit_per_hour = Gewinn / Stunde
calculator.server.stat.bargain_bonus = Feilschbonus
calculator.server.stat.sale_multiplier = Verkaufsmultiplikator

calculator.server.panel.expand_overlay_proposal = Overlay-Vorschlag aufklappen
calculator.server.panel.collapse_overlay_proposal = Overlay-Vorschlag einklappen

calculator.server.helper.mastery = Gib deine zusammengefasste Angel-Meisterschaft direkt ein.
calculator.server.helper.mastery_formula_prefix = Meisterschaft
calculator.server.helper.mastery_formula_suffix = steuert die direkte Preisraten-Formel vor der Normalisierung.
calculator.server.helper.before_zone_group_normalization = vor der Zonen-Gruppen-Normalisierung
calculator.server.helper.target_amount = Geschätzte Zeit bis zu dieser Menge.
calculator.server.helper.within_current_session_duration = innerhalb der aktuellen Sitzungsdauer
calculator.server.helper.select_target_fish = Wähle einen Zielfisch.
calculator.server.helper.target_status_per_day = {$label} · {$per_day}/Tag
calculator.server.helper.target_pmf_count = Zeigt die diskrete Ergebnisverteilung bis zu dieser Fanganzahl; der letzte Bucket fasst jedes höhere Ergebnis zusammen.
calculator.server.helper.target_pmf_auto_short = 0 = automatisch
calculator.server.helper.target_select_zone_item = Wähle einen Zielfisch oder Beutegegenstand aus dieser Zone.
calculator.server.helper.target_per_day_at_spot = {$per_day} / Tag am aktuellen Spot mit der aktuellen Konfiguration.
calculator.server.helper.target_missing_at_spot = Dieses Ziel erscheint aktuell nicht an diesem Spot.
calculator.server.helper.target_pmf_auto = 0 = automatisch. Der letzte PMF-Bucket ist aktuell ≥{$count} (0,5-%-Schwanzgrenze).
calculator.server.helper.target_pmf_fixed = Der letzte PMF-Bucket ist ≥{$count}.
calculator.server.helper.normalize_rates = Raten normalisieren
calculator.server.helper.distance_bonus = manueller %-Bonus, im Verkaufsmodell auf +150 % gedeckelt
calculator.server.helper.trade_price_curve = manuelle %-Kurve, häufig etwa 105–130 %
calculator.server.helper.apply_trade_settings = Handelseinstellungen anwenden
calculator.server.helper.food_family = Pro Nahrungskategorie wirkt immer nur eine Familie gleichzeitig. Höherstufige Nahrung ersetzt niedrigerstufige Nahrung derselben Familie.
calculator.server.helper.buff_group = Wenn du einen anderen Buff derselben Buff-Gruppe auswählst, ersetzt er den bisherigen.
calculator.server.helper.fish_only_notice = Gilt nur für Fische. Nicht-Fisch-Beute bleibt erhalten. Rote Fische werden immer behalten.
calculator.server.helper.using = verwendet
calculator.server.helper.per_cast = pro Wurf
calculator.server.helper.sale = Verkauf

calculator.server.tab.groups = Gruppen
calculator.server.tab.silver = Silber
calculator.server.tab.loot_flow = Beutefluss
calculator.server.tab.target_fish = Zielfisch
calculator.server.tab.overview = Übersicht
calculator.server.tab.inputs = Eingaben
calculator.server.tab.overlay = Overlay
calculator.server.tab.debug = Debug

calculator.server.discard.none = Nicht verwerfen
calculator.server.discard.white = Weiß
calculator.server.discard.green = Grün
calculator.server.discard.blue = Blau
calculator.server.discard.yellow = Gelb

calculator.server.target.expected = Erwartet ({$timespan})
calculator.server.target.time_to_target = Zeit bis zum Ziel
calculator.server.target.chance_at_least = Wahrscheinlichkeit für mindestens {$amount}
calculator.server.target.no_rows = Für die Zielanalyse sind an diesem Spot derzeit keine Beutezeilen verfügbar.
calculator.server.target.session_distribution_title = Sitzungsanzahl-Verteilung
calculator.server.target.session_distribution_description = Diskrete Sitzungs-Ergebnisverteilung für dieses Ziel innerhalb der aktuellen Sitzungsdauer.
calculator.server.target.count_bucket_probability = Wahrscheinlichkeits-Bucket nach Anzahl

calculator.server.chart.no_loot_rows = Für diese Zone sind noch keine quellgestützten Beutezeilen verfügbar.
calculator.server.chart.loot_flow_title = Beutefluss
calculator.server.chart.loot_flow_description = Jeder Fluss startet bei einer Fischgruppe, läuft durch quellgestützte Artenzeilen und wird dann zu silbergewichteten Gruppensummen zusammengeführt. Die linken Kennzahlen zeigen die Dropraten-Zusammensetzung, die rechten den Silberbeitrag.
calculator.server.chart.group_distribution_title = Gruppen-Dropratenverteilung
calculator.server.chart.group_distribution_description.normalized = Aktueller Fischgruppen-Anteil nach Gewichtung von Preisfisch, Selten und Hochwertig.
calculator.server.chart.group_distribution_description.raw = Rohe Fischgruppenraten nach Gewichtung von Preisfisch, Selten und Hochwertig. Diese Raten können zusammen über oder unter 100 % liegen.
calculator.server.chart.group_silver_distribution_title = Gruppen-Silberverteilung
calculator.server.chart.group_silver_distribution_description = Erwarteter Silberanteil je Fischgruppe nach Handels- und Preiseinstellungen.
calculator.server.chart.aria.group_distribution = Gruppen-Dropratenverteilung
calculator.server.chart.aria.group_silver_distribution = Gruppen-Silberverteilung
calculator.server.chart.aria.loot_flow = Erwarteter Beutefluss von Gruppen zu Beutezeilen
calculator.server.chart.aria.target_distribution = Zielfisch-Sitzungsverteilung
calculator.server.chart.aria.timeline = Angelzyklus-Zeitachse
calculator.server.chart.aria.distribution_tabs = Verteilungs-Tabs
calculator.server.chart.aria.top_level_tabs = Rechnerabschnitte

calculator.server.badge.aft = -{$percent}% AFT
calculator.server.badge.rare = +{$percent}% Selten
calculator.server.badge.hq = +{$percent}% HQ
calculator.server.badge.item_drr = +{$percent}% Gegenstands-DRR
calculator.server.badge.fish_multiplier = Fisch ×{$multiplier}
calculator.server.badge.fish_exp = +{$percent}% Angel-EP
calculator.server.badge.life_exp = +{$percent}% Lebens-EP
calculator.server.badge.set_effect = Set-Effekt
calculator.server.badge.level_drr = +{$percent}% Stufen-DRR

# Group and loot labels visible in charts and tooltips
calculator.server.group.prize = Preisfisch
calculator.server.group.rare = Selten
calculator.server.group.high_quality = Hochwertig
calculator.server.group.general = Allgemein
calculator.server.group.trash = Schrott
calculator.server.group.harpoon = Harpune
calculator.server.group.prize_curve_result = Ergebnis der Preisfisch-Kurve
calculator.server.group.prize_curve_result_detail = Direktes Preisgewicht vor der Normalisierung
calculator.server.group.zone_base_rate = Zonen-Basisrate
calculator.server.group.zone_base_rate_detail = Basisgruppenrate aus den Zonendaten
calculator.server.group.personal_overlay_raw_base_rate = Roh-Basisrate des persönlichen Overlays
calculator.server.group.personal_overlay_raw_base_rate_detail = Explizite rohe Basisgruppenrate aus dem aktuellen persönlichen Overlay-Vorschlag, bevor Gruppenboni addiert werden.
calculator.server.group.overlay_adjusted_normalized_share = Overlay-bereinigter normalisierter Anteil
calculator.server.group.overlay_adjusted_normalized_share_detail = Eine andere Rohgewichts-Überschreibung hat verändert, wie sich der normalisierte Anteil dieser Gruppe ergibt.
calculator.server.group.tooltip.overlay_explicit_with_bonus = Die rohe Overlay-Basisrate {$base} plus aktiver Gruppenbonus {$bonus} ergibt aktuell {$weight} Gesamt-Rohgewicht und {$share} normalisierten Anteil.
calculator.server.group.tooltip.overlay_explicit = Rohe Overlay-Basisrate {$base}. Aktueller normalisierter Anteil {$share}, nachdem die aktiven Gruppengewichte normalisiert wurden.
calculator.server.group.tooltip.overlay_adjusted = Das persönliche Overlay hat die aktiven Roh-Gruppengewichte verändert. {$group} ergibt aktuell {$share} normalisierten Anteil.
calculator.server.group.tooltip.source_backed_share = Quellgestützter Anteil der Gruppe {$group}
calculator.server.group.bonus.base_plus_bonus = Basis {$base} + Bonus {$bonus}
calculator.server.group.bonus.base_only = Basis {$base}
calculator.server.group.bonus.normalized_from_active_weights = Aus aktiven Rohgewichten normalisiert
calculator.server.group.bonus.mastery_raw_prize = Meisterschaft {$mastery} -> {$rate} roher Preiswert
calculator.server.group.bonus.rare = +{$rate}% Selten
calculator.server.group.bonus.high_quality = +{$rate}% HQ
calculator.server.group.bonus.none = Kein Bonus

calculator.server.chart.group_distribution_note.unavailable = Fischgruppendaten sind für diese Zone nicht verfügbar.
calculator.server.chart.group_distribution_note.overlay_active = Persönliche Überschreibungen der Roh-Basisraten für Gruppen sind aktiv. Bearbeitete Basiswerte erhalten zuerst alle aktiven Gruppenboni, danach werden die Gewichte in die aktuellen Gruppenanteile normalisiert.
calculator.server.chart.group_distribution_note.default = Zonengruppen werden nach Anwendung von Selten- und Hochwertig-Boni plus Preisgewicht aus Meisterschaft wieder auf 100 % normalisiert.

calculator.server.loot.tooltip.overlay.base_db_raw_rate = Basis-DB-Rohrate {$rate}.
calculator.server.loot.tooltip.overlay.explicit = Persönliche Overlay-Rohrate innerhalb der Gruppe {$base}. Aktuelle normalisierte Rate innerhalb der Gruppe {$rate}, nachdem die Rohwerte dieser Gruppe normalisiert wurden.{$base_detail}
calculator.server.loot.tooltip.overlay.added = Das persönliche Overlay hat diese Zeile zur Zone hinzugefügt. Sie ergibt aktuell die normalisierte Rate innerhalb der Gruppe {$rate}.{$base_detail}
calculator.server.loot.tooltip.overlay.changed = Das persönliche Overlay hat die Rohwerte dieser Gruppe verändert. Aktuelle normalisierte Rate innerhalb der Gruppe {$rate}.{$base_detail}
calculator.server.loot.tooltip.db_rate = DB {$rate}
calculator.server.loot.tooltip.community_guess = Community-Schätzwert {$rate}
calculator.server.loot.tooltip.derived = {$rate} aus den aktuellen Gruppengewichten abgeleitet
calculator.server.loot.tooltip.derived_total_expected_silver = {$share} des gesamten erwarteten Silbers abgeleitet
calculator.server.loot.note.unavailable = Erwartete Beutedaten sind für diese Zone nicht verfügbar.
calculator.server.loot.note.available = Die erwartete Beute verwendet durchschnittliche Sitzungswürfe, den aktuellen Fisch-Multiplikator, normalisierte Gruppenanteile und echte quellgestützte Gegenstandspreise. Artenzeilen zeigen DB-Raten innerhalb der Gruppe getrennt von Community-Schätzwerten für Preisfische und Community-Hinweisen zur Präsenz. Fisch-Autoverwerfen gilt nur für Fische, nicht für Nicht-Fisch-Beute.
calculator.server.loot.trade_sale_multiplier.off = Aus (×1)

calculator.server.zone_loot_summary.note.overlay_presence_without_groups = Ein persönlicher Overlay-Vorschlag ist für diese Zone aktiv. Präsenzunterstützung ist weiterhin verfügbar, auch wenn Rechner-Gruppenraten fehlen, deshalb bleiben ungelöste Zeilen mit Nicht zugewiesen sichtbar, wenn kein Slot bekannt ist.
calculator.server.zone_loot_summary.note.overlay_presence = Ein persönlicher Overlay-Vorschlag ist für diese Zone aktiv. Zeilen ohne aufgelösten Gruppenanteil oder Dropraten-Unterstützung bleiben sichtbar, bis ihre Struktur ergänzt oder der Overlay-Eintrag entfernt wird.
calculator.server.zone_loot_summary.note.overlay = Ein persönlicher Overlay-Vorschlag ist für diese Zone aktiv. Bearbeitete rohe Gruppen- und Gegenstandsraten werden in die hier gezeigte aktuelle Zonenzusammensetzung normalisiert.
calculator.server.zone_loot_summary.note.presence_without_groups = Für diese Zone ist Präsenzunterstützung verfügbar, aber Rechner-Gruppenraten fehlen. Zeilen ohne aufgelöste Gruppe oder Droprate bleiben gelistet und verwenden Nicht zugewiesen, wenn kein Slot bekannt ist.
calculator.server.zone_loot_summary.note.presence = Zeilen mit ungelöstem Gruppenanteil oder fehlender Dropraten-Unterstützung bleiben in der Liste sichtbar, bis ihre Struktur ergänzt wird.
calculator.server.zone_loot_summary.note.default = Gruppen folgen der aktuellen Rechner-Reihenfolge, und Zeilen zeigen die Droprate jedes Fischs oder Gegenstands innerhalb der Gruppe.
calculator.server.zone_loot_summary.note.unavailable.overlay = Erwartete Zonenbeutedaten sind für diese Zone selbst mit dem aktuellen persönlichen Overlay-Vorschlag nicht verfügbar.
calculator.server.zone_loot_summary.note.unavailable.default = Erwartete Zonenbeutedaten sind für diese Zone nicht verfügbar.
calculator.server.zone_loot_summary.profile.overlay = Persönlicher Overlay-Vorschlag
calculator.server.zone_loot_summary.profile.default = Rechner-Standardwerte
calculator.server.zone_loot_summary.title = Zonenfangprofil-Gruppen
calculator.server.zone_loot.method.fishing = Fischen
calculator.server.zone_loot.method.harpoon = Harpune
calculator.server.zone_loot.method_note.fishing = Rutenbasierte Zonengruppen und Artenraten innerhalb der Gruppe.
calculator.server.zone_loot.method_note.harpoon = Nur-Harpunen-Zonengruppen und Artenraten innerhalb der Gruppe.
calculator.server.zone_loot.conditions = Bedingungen
calculator.server.zone_loot.empty_group = Dieser Gruppe sind derzeit keine quellgestützten Artenzeilen zugeordnet.

calculator.server.value.unavailable = Nicht verfügbar

calculator.server.disclaimer.title = Warnung zur Datenqualität
calculator.server.disclaimer.p1 = Die derzeit verfügbaren Daten sind UNVOLLSTÄNDIG und manche Daten könnten VOLLSTÄNDIG FEHLEN.
calculator.server.disclaimer.p2 = Informationen zu Gruppenraten basieren auf älteren Daten und sind VERALTET.
calculator.server.disclaimer.p3 = Insbesondere: Preisfisch-Informationen basieren ausschließlich auf Community-Schätzungen und können völlig danebenliegen. Die echten Raten sind UNBEKANNT.
calculator.server.disclaimer.p4 = Wir versuchen, so genau wie möglich zu sein, aber derzeit solltest du nichts davon ungeprüft als Tatsache übernehmen.
calculator.server.disclaimer.p5 = In Zukunft wollen wir Daten per Crowdsourcing sammeln und freuen uns über spätere Beiträge.
