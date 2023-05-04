extends Button

@onready var player := %MediaPlayer
@onready var fileDialog: FileDialog = %FileDialog
@onready var synth: Synthesizer = %Synthesizer
@export var synth_volume := -25

func _on_Import_pressed():
	fileDialog.popup()

func _on_FileDialog_file_selected(path: String):
	synth.import(path)
	player.update()

func _on_url_entry_text_submitted(url: String):
	synth.eval("synth.setSong(\"%s\")" % url)
	player.update()
