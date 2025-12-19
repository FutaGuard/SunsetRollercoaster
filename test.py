import asyncio

from sunsetRollercoaster.component.mvdis import car_check

async def main():
    # r = await car_check.CarCheck().client.get('https://www.solarbus.com.tw')
    resp = await car_check.CarCheck().check_car('2',companyNo='96976746',plateNo='AVH-8180')
    print(resp)


asyncio.run(
   main()
)