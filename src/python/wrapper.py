import json
import sys

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

class Betrag:
    def __new__(wert, nachkomma, waehrung):
        self.wert = wert
        self.nachkomma = nachkomma
        self.waehrung = waehrung

class Regex:
    
    def __new__(regex):
        self.re = regex

    def matches(text): 
        return False # Boolean

    def find_in(text, index): 
        return None # Optional[String]

    def find_all(text): 
        return [""] # List[String]

    def replace_all(text, text_neu): 
        return "" # String

class FlurFlurstueck:

    def __init__(flur, flurstueck, gemarkung = None, teilflaeche_qm = None):
        self.flur = flur
        self.flurstueck = flurstueck
        self.gemarkung = gemarkung
        self.teilflaeche_qm = teilflaeche_qm

class Spalte1Eintrag:

    def __new__(lfd_nr, voll_belastet = True, nur_lastend_an = []):
        self.lfd_nr = lfd_nr
        self.voll_belastet = voll_belastet
        self.nur_lastend_an = nur_lastend_an # List[FlurFlurstueck]
    
    def get_lfd_nr():
        return self.lfd_nr
    
    def append_nur_lastend_an(nur_lastend_an: []):
        self.voll_belastet = False
        self.nur_lastend_an.append(nur_lastend_an)

def text_saubern(recht):
    pass

class PyResult:

    def err(self, string):
        self.type = "err"
        self.string = string
    
    def ok_string(self, string):
        self.type = "ok"
        self.string = string

    def get_string(self):
        return json.dumps(self)

def main():
    result = PyResult()
    result.err("invalid function")
    try:
        function_type = sys.argv[2]
        args_json = json.loads(sys.argv[3])
        if function_type == "text_saubern":
            string = text_saubern(args_json["recht"])
            result.ok_string(string)
        else:
            pass
    except BaseException as err:
        result.err(f"Unexpected {err=}, {type(err)=}")
    finally:
        print(result.get_string())