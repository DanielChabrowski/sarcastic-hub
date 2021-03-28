# Sarcastic-Hub

### Allow connections on port 9023 from specific subnet
```
sudo ufw allow from 192.168.1.0/24 to any port 9023
```

### Example config
```json
{
	"web_ui_address": "0.0.0.0:9023",
	"sink_management_address": "0.0.0.0:9024",

	"providers": [
		{
			"Filesystem": {
				"name": "Local music",
				"paths": [ "/home/USER/Music" ],
				"extensions": [ "mp3", "flac" ]
			}
		},
		{
			"Filesystem": {
				"name": "Local videos",
				"paths": [ "/home/USER/Videos/" ],
				"extensions": [ "mkv", "mp4" ]
			}
		}
	]
}
```
