import json
import sys
from json import JSONEncoder

def _default(self, obj):
    return getattr(obj.__class__, "to_json", _default.default)(obj)

_default.default = JSONEncoder().default
JSONEncoder.default = _default

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

class Regex(dict):
    
    def __init__(self, regex):
        self["re"] = regex

    def matches(self, text): 
        return False # Boolean

    def find_in(self, text, index): 
        return None # Optional[String]

    def find_all(self, text): 
        return [""] # List[String]

    def replace_all(self, text, text_neu): 
        return "" # String

class FlurFlurstueck(dict):

    def __init__(self, flur, flurstueck, gemarkung = None, teilflaeche_qm = None):
        self["flur"] = flur
        self["flurstueck"] = flurstueck
        self["gemarkung"] = gemarkung
        self["teilflaeche_qm"] = teilflaeche_qm

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

def text_saubern(recht):
    return recht

class PyResult(dict):

    def err(self, string):
        self.type = "err"
        self.string = string
    
    def ok_string(self, string):
        self.type = "ok"
        self.ok_type = "str"
        self.string = string

    def ok_list(self, list):
        self.type = "ok"
        self.ok_type = "list"
        self.list = list

    def ok_spalte1(self, spalte1):
        self.type = "ok"
        self.ok_type = "spalte1"
        self.spalte1 = spalte1
        
    def get_string(self):
        if self.type == "ok":
            if self.ok_type == "str":
                return "{\"result\": \"ok\", \"data\": { \"type\": \"str\", \"data\": \"" + self.string + "\" } }"
            elif self.ok_type == "list":
                return "{\"result\": \"ok\", \"data\": { \"type\": \"list\", \"data\": " + json.dumps(self.list) + " } }"
            elif self.ok_type == "spalte1":
                return "{\"result\": \"ok\", \"data\": { \"type\": \"spalte1\", \"data\": " + json.dumps(self.spalte1) + " } }"
        else:
            return "{\"result\": \"err\", \"data\": { \"text\": \"" + self.string + "\" } }"

def main():
    result = PyResult()
    result.err("invalid function")
    try:
        # eintrag = Spalte1Eintrag(5)
        # eintrag.append_nur_lastend_an([FlurFlurstueck("6", "275/4")])
        result.ok_spalte1(eintrag)
        function_type = sys.argv[2]
        args_json = json.loads(sys.argv[3])
        if function_type == "text_saubern":
            string = text_saubern(args_json["recht"])
            result.ok_string(string)
        # else:
        #     pass
    except BaseException as err:
        result.err(f"Unexpected {type(err)}: {err}")
    finally:
        print(result.get_string())

if __name__ == "__main__":
    main()