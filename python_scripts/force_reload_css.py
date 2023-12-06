from jinja2 import Environment, FileSystemLoader
import datetime 
import random 
import os
import shutil

current_time = datetime.datetime.now()  
formatted_date_time = current_time.strftime("%m%d%H%S")  
last_two_digits_time = random.randint(10, 99)
ten_digit_integer = formatted_date_time + str(last_two_digits_time)

home = os.path.expanduser("~")
origina_folder_path = os.path.join(home , "tunnel/raspberry-website/templates/base") #insert custom path to html files
enviroment = Environment(loader=FileSystemLoader(origina_folder_path))
html_files = [x for x in os.listdir(origina_folder_path) if x[-4:] == "html"] 

custom_folder_path = os.path.join(home, "tunnel/raspberry-website/templates/templates")

specific_file = "main.css"  # Replace this with the specific filename of stylesheet to keep

for filename in os.listdir(custom_folder_path):
    file_path = os.path.join(custom_folder_path, filename)
    if filename != specific_file and os.path.isfile(file_path):
        try:
            os.unlink(file_path)  
            print(f"Deleted: {file_path}")
        except Exception as e:
            print(f"Failed to delete {file_path}. Reason: {e}")

for (k, file) in enumerate(html_files):
    template = enviroment.get_template(file)
    content = template.render(query=ten_digit_integer)
    with open(os.path.join(custom_folder_path, file), mode="w") as f:
        f.write(content)
    print(f"{k}/{len(html_files)} converted")
