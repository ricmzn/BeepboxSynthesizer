extends Button

@onready var player: AudioStreamPlayer = %AudioStreamPlayer
@onready var fileDialog: FileDialog = %FileDialog
@onready var player_ui := %MediaPlayer
@export var synth_volume := -25

func _on_Import_pressed():
	fileDialog.popup()

func _on_FileDialog_file_selected(path: String):
	player.stream.import(path)
	player_ui.update()

func _on_url_entry_text_submitted(url: String):
	player.get_stream_playback().eval("synth.setSong(\"%s\")" % url)
	player_ui.update()
