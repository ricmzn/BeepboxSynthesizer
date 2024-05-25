extends Button

@onready var player: AudioStreamPlayer = %AudioStreamPlayer

var playing = false

func _on_Button_pressed():
	if playing:
		player.pause()
		playing = false
		text = "Play"
	else:
		player.resume()
		playing = true
		text = "Pause"
