# -*- coding: utf-8 -*-

import json
from re import compile
from json import JSONEncoder
import traceback
import base64

class Regex(dict):
    
    def __init__(self, regex):
        self["re"] = compile(regex)

    def matches(self, text): 
        return self["re"].match(text)

    def find_in(self, text, index): 
        matches = self["re"].findall(text)
        if index < len(matches):
            return matches[index]
        elif (len(matches) == 1 and index < len(matches[0])):
            return matches[0][index]
        else: 
            return None

    def find_all(self, text): 
        return self["re"].findall(text)

    def replace_all(self, text, text_neu):
        if self["re"].match(text):
            return self["re"].sub(text_neu, text)
        else:
            return text

## REGEX_LIST

def _default(self, obj):
    return getattr(obj.__class__, "to_json", _default.default)(obj)

_default.default = JSONEncoder().default
JSONEncoder.default = _default

speziell_vormerkung_index = None

class SchuldenArt(str):
    Grundschuld = 'Grundschuld'
    Hypothek = 'Hypothek'
    Rentenschuld = 'Rentenschuld'
    Aufbauhypothek = 'Aufbauhypothek'
    Sicherungshypothek = 'Sicherungshypothek'
    Widerspruch = 'Widerspruch'
    Arresthypothek = 'Arresthypothek'
    SicherungshypothekGem128ZVG = 'SicherungshypothekGem128ZVG'
    Hoechstbetragshypothek = 'Hoechstbetragshypothek'
    Sicherungsgrundschuld = 'Sicherungsgrundschuld'
    Zwangssicherungshypothek = 'Zwangssicherungshypothek'
    NichtDefiniert = 'NichtDefiniert'

# TODO: RechteArt.SpeziellVormerkung(index)
class RechteArt(str):
    Abwasserleitungsrecht = 'Abwasserleitungsrecht'
    Auflassungsvormerkung = 'Auflassungsvormerkung'
    Ausbeutungsrecht = 'Ausbeutungsrecht'
    AusschlussDerAufhebungDerGemeinschaftGem1010BGB = 'AusschlussDerAufhebungDerGemeinschaftGem1010BGB'
    Baubeschraenkung = 'Baubeschraenkung'
    Bebauungsverbot = 'Bebauungsverbot'
    Benutzungsrecht = 'Benutzungsrecht'
    BenutzungsregelungGem1010BGB = 'BenutzungsregelungGem1010BGB'
    Bepflanzungsverbot = 'Bepflanzungsverbot'
    Bergschadenverzicht = 'Bergschadenverzicht'
    Betretungsrecht = 'Betretungsrecht'
    Bewässerungsrecht = 'Bewässerungsrecht'
    BpD = 'BpD'
    BesitzrechtNachEGBGB = 'BesitzrechtNachEGBGB'
    BohrUndSchuerfrecht = 'BohrUndSchuerfrecht'
    Brunnenrecht = 'Brunnenrecht'
    Denkmalschutz = 'Denkmalschutz'
    DinglichesNutzungsrecht = 'DinglichesNutzungsrecht'
    DuldungVonEinwirkungenDurchBaumwurf = 'DuldungVonEinwirkungenDurchBaumwurf'
    DuldungVonFernmeldeanlagen = 'DuldungVonFernmeldeanlagen'
    Durchleitungsrecht = 'Durchleitungsrecht'
    EinsitzInsitzrecht = 'EinsitzInsitzrecht'
    Entwasserungsrecht = 'Entwasserungsrecht'
    Erbbaurecht = 'Erbbaurecht'
    Erwerbsvormerkung = 'Erwerbsvormerkung'
    Fensterrecht = 'Fensterrecht'
    Fensterverbot = 'Fensterverbot'
    Fischereirecht = 'Fischereirecht'
    Garagenrecht = 'Garagenrecht'
    Gartenbenutzungsrecht = 'Gartenbenutzungsrecht'
    GasleitungGasreglerstationFerngasltg = 'GasleitungGasreglerstationFerngasltg'
    GehWegeFahrOderLeitungsrecht = 'GehWegeFahrOderLeitungsrecht'
    Gewerbebetriebsbeschrankung = 'Gewerbebetriebsbeschrankung'
    GewerblichesBenutzungsrecht = 'GewerblichesBenutzungsrecht'
    Grenzbebauungsrecht = 'Grenzbebauungsrecht'
    Grunddienstbarkeit = 'Grunddienstbarkeit'
    Hochspannungsleitungsrecht = 'Hochspannungsleitungsrecht'
    Immissionsduldungsverpflichtung = 'Immissionsduldungsverpflichtung'
    Insolvenzvermerk = 'Insolvenzvermerk'
    Kabelrecht = 'Kabelrecht'
    Kanalrecht = 'Kanalrecht'
    Kiesabbauberechtigung = 'Kiesabbauberechtigung'
    Kraftfahrzeugabstellrecht = 'Kraftfahrzeugabstellrecht'
    LeibgedingAltenteilsrechtAuszugsrecht = 'LeibgedingAltenteilsrechtAuszugsrecht'
    LeitungsOderAnlagenrecht = 'LeitungsOderAnlagenrecht'
    Mauerrecht = 'Mauerrecht'
    Mitbenutzungsrecht = 'Mitbenutzungsrecht'
    Mobilfunkstationsrecht = 'Mobilfunkstationsrecht'
    Muehlenrecht = 'Muehlenrecht'
    Mulltonnenabstellrecht = 'Mulltonnenabstellrecht'
    Nacherbenvermerk = 'Nacherbenvermerk'
    Niessbrauchrecht = 'Niessbrauchrecht'
    Nutzungsbeschrankung = 'Nutzungsbeschrankung'
    Pfandung = 'Pfandung'
    Photovoltaikanlagenrecht = 'Photovoltaikanlagenrecht'
    Pumpenrecht = 'Pumpenrecht'
    Reallast = 'Reallast'
    RegelungUeberDieHöheDerNotwegrenteGemaess912Bgb = 'RegelungUeberDieHöheDerNotwegrenteGemaess912Bgb'
    RegelungUeberDieHöheDerUeberbaurenteGemaess912Bgb = 'RegelungUeberDieHöheDerUeberbaurenteGemaess912Bgb'
    Rueckauflassungsvormerkung = 'Rueckauflassungsvormerkung'
    Ruckerwerbsvormerkung = 'Ruckerwerbsvormerkung'
    Sanierungsvermerk = 'Sanierungsvermerk'
    Schachtrecht = 'Schachtrecht'
    SonstigeDabagrechteart = 'SonstigeDabagrechteart'
    SonstigeRechte = 'SonstigeRechte'
    Tankstellenrecht = 'Tankstellenrecht'
    Testamentsvollstreckervermerk = 'Testamentsvollstreckervermerk'
    Transformatorenrecht = 'Transformatorenrecht'
    Ueberbaurecht = 'Ueberbaurecht'
    UebernahmeVonAbstandsflachen = 'UebernahmeVonAbstandsflachen'
    Umlegungsvermerk = 'Umlegungsvermerk'
    Umspannanlagenrecht = 'Umspannanlagenrecht'
    Untererbbaurecht = 'Untererbbaurecht'
    VerausserungsBelastungsverbot = 'VerausserungsBelastungsverbot'
    Verfuegungsverbot = 'Verfuegungsverbot'
    VerwaltungsUndBenutzungsregelung = 'VerwaltungsUndBenutzungsregelung'
    VerwaltungsregelungGem1010Bgb = 'VerwaltungsregelungGem1010Bgb'
    VerzichtAufNotwegerente = 'VerzichtAufNotwegerente'
    VerzichtAufUeberbaurente = 'VerzichtAufUeberbaurente'
    Viehtrankerecht = 'Viehtrankerecht'
    Viehtreibrecht = 'Viehtreibrecht'
    Vorkaufsrecht = 'Vorkaufsrecht'
    Wasseraufnahmeverpflichtung = 'Wasseraufnahmeverpflichtung'
    Wasserentnahmerecht = 'Wasserentnahmerecht'
    Weiderecht = 'Weiderecht'
    Widerspruch = 'Widerspruch'
    Windkraftanlagenrecht = 'Windkraftanlagenrecht'
    Wohnrecht = 'Wohnrecht'
    WohnungsOderMitbenutzungsrecht = 'WohnungsOderMitbenutzungsrecht'
    Wohnungsbelegungsrecht = 'Wohnungsbelegungsrecht'
    WohnungsrechtNach1093Bgb = 'WohnungsrechtNach1093Bgb'
    Zaunerrichtungsverbot = 'Zaunerrichtungsverbot'
    Zaunrecht = 'Zaunrecht'
    Zustimmungsvorbehalt = 'Zustimmungsvorbehalt'
    Zwangsversteigerungsvermerk = 'Zwangsversteigerungsvermerk'
    Zwangsverwaltungsvermerk = 'Zwangsverwaltungsvermerk'

    def SpeziellVormerkung(rechteverweis):
        global speziell_vormerkung_index
        speziell_vormerkung_index = rechteverweis
        return 'SpeziellVormerkung'

class Waehrung(str):
    Euro = 'Euro'
    DMark = 'DMark'
    MarkDDR = 'MarkDDR'
    Goldmark = 'Goldmark'
    Rentenmark = 'Rentenmark'
    Reichsmark = 'Reichsmark'
    GrammFeingold = 'GrammFeingold'

class Betrag(dict):
    def __init__(self, wert, nachkomma, waehrung):
        self["wert"] = wert
        self["nachkomma"] = nachkomma
        self["waehrung"] = waehrung

class FlurFlurstueck(dict):

    def __init__(self, flur, flurstueck, gemarkung = None, teilflaeche_qm = None):
        self["flur"] = flur
        self["flurstueck"] = flurstueck
        self["gemarkung"] = gemarkung
        self["teilflaeche_qm"] = teilflaeche_qm

class Spalte1Eintraege(dict):

    def __init__(self, eintraege = []):
        self.eintraege = eintraege # List[Spalte1Eintrag]
        self.warnungen = []

    def __len__(self):
        return len(self.eintraege)
    
    def __getitem__(self, index):
        return self.eintraege[index]

    def __setitem__(self, key, value):
        self.eintraege[key] = value

    def append(self, eintrag):
        self.eintraege.append(eintrag)

    def warnung(self, warnung):
        self.warnungen.append(warnung)

class Spalte1Eintrag(dict):

    def __init__(self, lfd_nr, voll_belastet = True, nur_lastend_an = []):
        self["lfd_nr"] = lfd_nr
        self["voll_belastet"] = voll_belastet
        self["nur_lastend_an"] = nur_lastend_an # List[FlurFlurstueck]

    def get_lfd_nr(self):
        return self["lfd_nr"]
    
    def append_nur_lastend_an(self, nur_lastend_an = []):
        self["voll_belastet"] = False
        self["nur_lastend_an"].extend(nur_lastend_an)

class PyResult(dict):
    
    def err(self, any):
        self.type = "err"
        self.err = any
        self.rechteart = False
        self.schuldenart = False
        return self

    def ok(self, any, ra, sa):
        self.type = "ok"
        self.ok = any
        self.rechteart = ra
        self.schuldenart = sa
        return self

    def get_string(self):
        if self.type == "ok":
            if self.rechteart:
                if speziell_vormerkung_index is not None:
                    return "{\"result\": \"ok\", \"data\": { \"type\": \"rechteart\", \"data\": { \"SpeziellVormerkung\": { \"rechteverweis\": " + str(speziell_vormerkung_index) + " } } } }"
                else: 
                    return "{\"result\": \"ok\", \"data\": { \"type\": \"rechteart\", \"data\": " + json.dumps(self.ok) + " } }"
            elif self.schuldenart:
                return "{\"result\": \"ok\", \"data\": { \"type\": \"schuldenart\", \"data\": " + json.dumps(self.ok) + " } }"
            elif isinstance(self.ok, str):
                return "{\"result\": \"ok\", \"data\": { \"type\": \"str\", \"data\": \"" + self.ok + "\" } }"
            elif isinstance(self.ok, list):
                return "{\"result\": \"ok\", \"data\": { \"type\": \"list\", \"data\": " + json.dumps(self.ok) + " } }"
            elif isinstance(self.ok, Spalte1Eintraege):
                return "{\"result\": \"ok\", \"data\": { \"type\": \"spalte1\", \"data\": { \"eintraege\": " + json.dumps(self.ok.eintraege) + ", \"warnungen\": " + json.dumps(self.ok.warnungen) + "} } }"
            elif isinstance(self.ok, Betrag):
                return "{\"result\": \"ok\", \"data\": { \"type\": \"betrag\", \"data\": " + json.dumps(self.ok) + " } }"
            else:
                return "{\"result\": \"ok\", \"data\": { \"type\": \"???\", \"data\": \"\" } }"
        else:
            return "{\"result\": \"err\", \"data\": { \"text\": \"" + self.err + "\" } }"

def main_func():
## MAIN_SCRIPT
    pass

def main():
    result = PyResult().err("invalid function")

    ra = False 
    sa = False
    ## RA_SA

    try:
        return_val = main_func()
        result = PyResult().ok(return_val, ra, sa)
    except Exception as ex:
        tb = "".join(traceback.TracebackException.from_exception(ex).format())
        tb = tb.replace("\"", "'").replace("\r\n", "⣿").replace("\n", "⣿")
        tb = tb.encode("utf-8")
        tb = base64.b64encode(tb)
        tb = tb.decode("utf-8")
        result = PyResult().err(tb)
    finally:
        result_str = result.get_string()
        print(u'' + result_str)

if __name__ == "__main__":
    main()